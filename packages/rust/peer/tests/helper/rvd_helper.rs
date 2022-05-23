use common::messages::rvd::{DisplayChangeReceived, ProtocolVersionResponse, RvdMessage};
use peer::rvd::{RvdClientHandler, RvdHostHandler};

pub fn handshake(host: Option<&mut RvdHostHandler>, client: Option<&mut RvdClientHandler>) {
    let mut write = Vec::new();
    let mut events = Vec::new();

    if let Some(client) = client {
        let protocol_message = RvdHostHandler::protocol_version();

        client
            ._handle(protocol_message, &mut write, &mut events)
            .expect("handler failed");
    }

    if let Some(host) = host {
        let msg = RvdMessage::ProtocolVersionResponse(ProtocolVersionResponse { ok: true });
        host._handle(msg, &mut events).expect("handler failed");
        let msg = RvdMessage::DisplayChangeReceived(DisplayChangeReceived {});
        host._handle(msg, &mut events).expect("handler failed");
    }
}
