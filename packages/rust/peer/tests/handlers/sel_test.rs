use common::messages::sel::{SelMessage, TransportDataServerMessageUnreliable};
use peer::{
    hash,
    services::{
        helpers::{cipher_unreliable_peer::CipherUnreliablePeer, crypto::kdf2},
        sel_handler::SelHandler,
    },
};

#[test]
fn sel_reliable() {
    let mut handler = SelHandler::new();
    let vec = vec![10, 20, 30];
    let message = SelHandler::wrap_reliable(vec.clone());

    assert_eq!(handler.handle(message).unwrap(), vec);
}


#[test]
#[should_panic]
fn sel_try_reliable_before_ready() {
    let handler = SelHandler::new();
    handler.unreliable_cipher();
}

#[test]
fn sel_unreliable() {
    let mut handler = SelHandler::new();

    // sizes just for testing this is obviously insecure

    let session_id = &[1u8; 16];
    let peer_id = [2u8; 16];
    let peer_key = &[3u8; 16];

    let send_data = vec![9, 0, 5, 1];


    // swapped for receiving side
    let (receive_key, send_key) = kdf2(hash!(session_id, &peer_id, peer_key));
    let server_cipher = CipherUnreliablePeer::new(send_key.to_vec(), receive_key.to_vec());

    handler.derive_unreliable(session_id, &peer_id, peer_key);

    let handler_cipher = handler.unreliable_cipher();
    let message =
        match SelHandler::wrap_unreliable(send_data.clone(), peer_id, handler_cipher).unwrap() {
            SelMessage::TransportDataPeerMessageUnreliable(msg) => msg,
            _ => panic!("bad message returned"),
        };


    assert_eq!(message.peer_id, peer_id);

    // confirm sel does encryption properly by checking decryption
    assert_eq!(
        server_cipher
            .decrypt(&message.data, message.counter)
            .unwrap(),
        send_data
    );

    let message = match SelHandler::wrap_unreliable(send_data.clone(), peer_id, &server_cipher)
        .unwrap()
    {
        SelMessage::TransportDataPeerMessageUnreliable(msg) =>
            SelMessage::TransportDataServerMessageUnreliable(TransportDataServerMessageUnreliable {
                data: msg.data,
                counter: msg.counter,
            }),
        _ => panic!("bad message returned"),
    };

    assert_eq!(handler.handle(message).unwrap(), send_data);
}
