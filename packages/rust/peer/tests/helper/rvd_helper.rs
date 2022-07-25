use common::messages::rvd::{
    DisplayShareAck,
    HandshakeComplete,
    ProtocolVersionResponse,
    RvdMessage,
    UnreliableAuthFinal,
    UnreliableAuthInitial,
    UnreliableAuthInter,
};
use peer::rvd::{RvdClientHandler, RvdHostHandler};

pub fn handshake(host: Option<&mut RvdHostHandler>, client: Option<&mut RvdClientHandler>) {
    let mut write = Vec::new();
    let mut events = Vec::new();

    if let Some(client) = client {
        let msg = RvdMessage::ProtocolVersionResponse(ProtocolVersionResponse { ok: true });
        client
            ._handle(msg, &mut write, &mut events)
            .expect("handler failed");
        let msg = write.remove(0);
        let challange = match msg {
            RvdMessage::UnreliableAuthInitial(UnreliableAuthInitial { challenge, .. }) => challenge,
            _ => panic!("wrong message type"),
        };
        client
            ._handle(
                RvdMessage::UnreliableAuthInter(UnreliableAuthInter {
                    challenge: [0u8; 16],
                    response: challange,
                }),
                &mut write,
                &mut events,
            )
            .expect("handler failed");
        client
            ._handle(
                RvdMessage::HandshakeComplete(HandshakeComplete {}),
                &mut write,
                &mut events,
            )
            .expect("handler failed");
    }

    if let Some(host) = host {
        let protocol_message = RvdClientHandler::protocol_version();

        host._handle(protocol_message, &mut write, &mut events)
            .expect("handler failed");
        write.clear();
        host._handle(
            RvdMessage::UnreliableAuthInitial(UnreliableAuthInitial {
                challenge: *b"challengechallen",
                zero: [0u8; 16],
            }),
            &mut write,
            &mut events,
        )
        .expect("handler failed");
        let msg = write.remove(0);
        let challenge = match msg {
            RvdMessage::UnreliableAuthInter(UnreliableAuthInter { challenge, .. }) => challenge,
            _ => panic!("wrong message type"),
        };
        host._handle(
            RvdMessage::UnreliableAuthFinal(UnreliableAuthFinal {
                response: challenge,
            }),
            &mut write,
            &mut events,
        )
        .expect("handler failed");
    }
}
