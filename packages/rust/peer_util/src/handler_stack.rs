use common::messages::{
    rvd::{AccessMask, DisplayId, RvdMessage},
    svsc::{Cookie, LeaseId},
    wpskka::AuthSchemeType,
    ChanneledMessage,
};
use io::{
    IoHandle,
    Reliable,
    SendError,
    Source,
    TransportError,
    TransportResponse,
    TransportResult,
    Unreliable,
    UnreliableState,
};
use peer::{
    higher_handler::{HigherError, HigherHandlerClient, HigherHandlerHost, HigherHandlerTrait},
    lower::{LowerError, LowerHandlerSignal, LowerHandlerTrait},
    wpskka::{WpskkaClientError, WpskkaError, WpskkaHostError},
    InformEvent,
};
use rtp::packet::Packet;
use webrtc_util::{Marshal, MarshalSize};

macro_rules! send {
    ($self: ident, $message: expr) => {
        let higher_output = $self.higher.send($message)?;
        let lower_output = $self.lower.send(higher_output)?;
        $self.io_handle.send(lower_output)?;
    };
}


pub struct HandlerStack<H, L, R, U> {
    higher: H,
    lower: L,
    pub io_handle: IoHandle<R, U>,
    unreliable_needs_connect: bool,
}

impl<H, L, R, U> HandlerStack<H, L, R, U>
where
    H: HigherHandlerTrait,
    L: LowerHandlerTrait,
    R: Reliable,
    U: Unreliable,
{
    pub fn new(higher: H, lower: L, io_handle: IoHandle<R, U>) -> Self {
        Self {
            higher,
            lower,
            io_handle,
            unreliable_needs_connect: true,
        }
    }

    pub fn handle_next_message(&mut self) -> Option<Result<Vec<InformEvent>, HandlerError>> {
        let result = self.io_handle.recv()?;
        Some(self.handle_next_message_internal(result))
    }

    fn handle_next_message_internal(
        &mut self,
        result: TransportResult,
    ) -> Result<Vec<InformEvent>, HandlerError> {
        let (to_wire, informs) = match result {
            Ok(TransportResponse::ReliableMessage(message)) => self.handle(&message)?,
            Ok(TransportResponse::UnreliableMessage(message, addr)) => {
                let res = self.handle(&message)?;

                // If we have not already obtained the remote address to which to send UDP packets,
                // and we receive a UDP packet which has been successfully authenticated
                if self.unreliable_needs_connect {
                    let unreliable_state = self.io_handle.unreliable_state();

                    if unreliable_state == UnreliableState::Connected {
                        self.unreliable_needs_connect = false;
                    } else {
                        self.io_handle.connect_unreliable(&addr).map_err(|error| {
                            HandlerError::Transport(TransportError::Fatal {
                                source: Source::WriteReliable,
                                error,
                            })
                        })?;
                        self.unreliable_needs_connect = false;
                    }
                }

                res
            }
            Ok(TransportResponse::Shutdown(source)) => (vec![], vec![]), // TODO InformEvent::TransportShutdown(source)
            Err(error) => return Err(error.into()),
        };

        for wire_msg in to_wire {
            self.io_handle.send(wire_msg)?;
        }

        Ok(informs)
    }

    pub fn handle(
        &mut self,
        wire: &[u8],
    ) -> Result<(Vec<ChanneledMessage<Vec<u8>>>, Vec<InformEvent>), HandlerError> {
        // in lower: wire(wpskaa | sel) --> wpskka
        // in higher: wpskka --> wpskaa
        // out lower: wpskak --> wire(wpskaa | sel)

        // if we are direct, wire is wpskaa data
        // if we are signal, wire data is sel

        let mut send = Vec::new();
        let mut inform_events = Vec::new();


        let mut data_for_higher = Vec::new();
        // if direct, lower just passes us back
        // if signal it decrypts it and passes is through svsc
        // regardless we will get wpskka data out of ths
        inform_events.append(&mut self.lower.handle(wire, &mut data_for_higher, &mut send)?);

        for msg in send {
            self.io_handle.send(msg)?;
        }

        // we get wpskka data from higher
        let mut wpskka_outputs = Vec::new();
        inform_events.append(&mut self.higher.handle(&data_for_higher, &mut wpskka_outputs)?);


        let mut to_wire = Vec::new();

        for wpskka_output in wpskka_outputs {
            to_wire.push(self.lower.send(wpskka_output)?);
        }

        Ok((to_wire, inform_events))
    }
}

impl<H, R, U> HandlerStack<H, LowerHandlerSignal, R, U>
where
    R: Reliable,
    U: Unreliable,
{
    pub fn establish_session_request(&mut self, lease_id: LeaseId) -> Result<(), HandlerError> {
        let message = self.lower.establish_session_request(lease_id);
        let data = self.lower.send(message)?;
        self.io_handle.send(data).map_err(Into::into)
    }

    pub fn lease_request(&mut self, cookie: Option<Cookie>) -> Result<(), HandlerError> {
        let message = self.lower.lease_request(cookie);
        let data = self.lower.send(message)?;
        self.io_handle.send(data).map_err(Into::into)
    }

    pub fn lease_id(&self) -> LeaseId {
        self.lower.lease().expect("no lease data").id
    }
}

impl<L, R, U> HandlerStack<HigherHandlerClient, L, R, U>
where
    L: LowerHandlerTrait,
    R: Reliable,
    U: Unreliable,
{
    pub fn protocol_version(&mut self) -> Result<(), HandlerError> {
        let msg = self.higher.protocol_version();
        send!(self, msg);
        Ok(())
    }

    pub fn process_password(&mut self, password: &[u8]) -> Result<(), HandlerError> {
        let message = self.higher.process_password(password)?;
        let higher_output = self.higher.send(message)?;
        let lower_output = self.lower.send(higher_output)?;
        self.io_handle.send(lower_output)?;
        Ok(())
    }

    pub fn try_auth(&mut self, scheme: AuthSchemeType) -> Result<(), HandlerError> {
        let message = self.higher.try_auth(scheme);
        send!(self, message);
        Ok(())
    }
}

impl<L, R, U> HandlerStack<HigherHandlerHost, L, R, U>
where
    L: LowerHandlerTrait,
    R: Reliable,
    U: Unreliable,
{
    pub fn key_exchange(&mut self) -> Result<(), HandlerError> {
        let message = self.higher.key_exchange()?;
        let higher_output = self.higher.send(message)?;
        let lower_output = self.lower.send(higher_output)?;
        self.io_handle.send(lower_output)?;
        Ok(())
    }

    pub fn set_static_password(&mut self, static_password: Option<Vec<u8>>) {
        self.higher.set_static_password(static_password)
    }

    pub fn share_display(
        &mut self,
        name: String,
        access: AccessMask,
    ) -> Result<DisplayId, HandlerError> {
        let (display_id, message) = self.higher.share_display(name, access)?; // TODO this should send messages instead of returning
        send!(self, message);
        Ok(display_id)
    }

    pub fn unshare_display(&mut self, display_id: DisplayId) -> Result<(), HandlerError> {
        let message = self.higher.unshare_display(display_id)?;
        send!(self, message);
        Ok(())
    }

    pub fn send_frame_update(
        &mut self,
        display_id: DisplayId,
        fragments: impl Iterator<Item = Packet>,
    ) -> Result<(), HandlerError> {
        let mut buffer = Vec::<u8>::new();

        for packet in fragments {
            let size = MarshalSize::marshal_size(&packet);
            if buffer.len() < size {
                buffer.resize(size, 0);
            }
            let written = packet
                .marshal_to(&mut buffer[..])
                .expect("Insufficient buffer size");
            let data = &buffer[.. written];
            let message = HigherHandlerHost::frame_update(display_id, data);
            send!(self, message);
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("transport-layer error: {0}")]
    Transport(#[from] TransportError),
    #[error("lower-layer error: {0}")]
    Lower(#[from] LowerError),
    #[error("higher-layer error: {0}")]
    Higher(#[from] HigherError),
    #[error("failed to send message to IO handler: {0}")]
    SendError(#[from] SendError),
}
