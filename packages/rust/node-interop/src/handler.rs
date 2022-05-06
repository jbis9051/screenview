use peer::{
    handler_stack::{HandlerError, HandlerStack},
    higher_handler::HigherHandler,
    io::{IoHandle, TcpHandle, UdpHandle},
    lower::{LowerHandlerDirect, LowerHandlerSignal},
    rvd::{RvdClientHandler, RvdHostHandler},
    wpskka::{WpskkaClientHandler, WpskkaHostHandler},
    InformEvent,
};

type HStack<W, R, L> = HandlerStack<HigherHandler<W, R>, L, TcpHandle, UdpHandle>;

pub enum ScreenViewHandler {
    HostSignal(HStack<WpskkaHostHandler, RvdHostHandler, LowerHandlerSignal>),
    HostDirect(HStack<WpskkaHostHandler, RvdHostHandler, LowerHandlerDirect>),
    ClientSignal(HStack<WpskkaClientHandler, RvdClientHandler, LowerHandlerSignal>),
    ClientDirect(HStack<WpskkaClientHandler, RvdClientHandler, LowerHandlerDirect>),
}

impl ScreenViewHandler {
    pub fn new_host_signal() -> Self {
        Self::HostSignal(HandlerStack::new(
            HigherHandler::<WpskkaHostHandler, _>::new(),
            LowerHandlerSignal::new(),
            IoHandle::new(),
        ))
    }

    pub fn new_host_direct() -> Self {
        Self::HostDirect(HandlerStack::new(
            HigherHandler::<WpskkaHostHandler, _>::new(),
            LowerHandlerDirect::new(),
            IoHandle::new(),
        ))
    }

    pub fn new_client_signal() -> Self {
        Self::ClientSignal(HandlerStack::new(
            HigherHandler::<WpskkaClientHandler, _>::new(),
            LowerHandlerSignal::new(),
            IoHandle::new(),
        ))
    }

    pub fn new_client_direct() -> Self {
        Self::ClientDirect(HandlerStack::new(
            HigherHandler::<WpskkaClientHandler, _>::new(),
            LowerHandlerDirect::new(),
            IoHandle::new(),
        ))
    }

    pub fn io_handle(&mut self) -> &mut IoHandle<TcpHandle, UdpHandle> {
        match self {
            Self::HostSignal(stack) => stack.io_handle(),
            Self::HostDirect(stack) => stack.io_handle(),
            Self::ClientSignal(stack) => stack.io_handle(),
            Self::ClientDirect(stack) => stack.io_handle(),
        }
    }

    pub fn handle_next_message(&mut self) -> Option<Result<Vec<InformEvent>, HandlerError>> {
        match self {
            Self::HostSignal(stack) => stack.handle_next_message(),
            Self::HostDirect(stack) => stack.handle_next_message(),
            Self::ClientSignal(stack) => stack.handle_next_message(),
            Self::ClientDirect(stack) => stack.handle_next_message(),
        }
    }
}
