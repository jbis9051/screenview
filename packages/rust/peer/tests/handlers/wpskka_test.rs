use common::messages::wpskka::{
    TransportDataMessageReliable,
    TransportDataMessageUnreliable,
    WpskkaMessage,
};
use peer::services::{
    wpskka::{WpskkaClientHandler, WpskkaClientInform, WpskkaHostHandler, WpskkaHostInform},
    InformEvent,
};


fn srp_authenticate(
    host: &mut WpskkaHostHandler,
    client: &mut WpskkaClientHandler,
    password: &[u8],
) {
    let mut events = Vec::new();
    let mut write = Vec::new();

    let auth_schemes = host.auth_schemes().expect("handler failed");

    assert!(client
        .handle(auth_schemes, &mut write, &mut events)
        .expect("handler failed")
        .is_none());
    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 0);
    assert!(matches!(write[0], WpskkaMessage::TryAuth(_)));

    // TryAuth
    let message = write.remove(0);

    assert!(host
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());
    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 0);
    assert!(matches!(write[0], WpskkaMessage::AuthMessage(_)));

    // HostHello
    let message = write.remove(0);

    assert!(client
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::WpskkaClientInform(WpskkaClientInform::PasswordPrompt)
    ));

    events.clear();

    client.process_password(&password, &mut write).expect("");
    assert_eq!(write.len(), 1);
    assert!(matches!(write[0], WpskkaMessage::AuthMessage(_)));

    // ClientHello
    let message = write.remove(0);
    assert!(host
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());
    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::WpskkaHostInform(WpskkaHostInform::AuthSuccessful)
    ));

    events.clear();

    // HostVerify
    let message = write.remove(0);
    assert!(client
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::WpskkaClientInform(WpskkaClientInform::AuthSuccessful)
    ));
}

fn test_reliable_communication(host: &mut WpskkaHostHandler, client: &mut WpskkaClientHandler) {
    let data = vec![9, 0, 5, 1];

    // test host encrypt, client decrypt
    let host_cipher = host.reliable_cipher_mut();
    let message = WpskkaMessage::TransportDataMessageReliable(TransportDataMessageReliable {
        data: host_cipher.encrypt(&data).unwrap(),
    });
    let mut write = Vec::new();
    let mut events = Vec::new();
    let result = client
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .expect("expected data");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);
    assert_eq!(&result, &data);

    // test client encrypt, host decrypt
    let client_cipher = client.reliable_cipher_mut();
    let message = WpskkaMessage::TransportDataMessageReliable(TransportDataMessageReliable {
        data: client_cipher.encrypt(&data).unwrap(),
    });
    let mut write = Vec::new();
    let mut events = Vec::new();
    let result = host
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .expect("expected data");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);
    assert_eq!(&result, &data);
}

fn test_unreliable_communication(host: &mut WpskkaHostHandler, client: &mut WpskkaClientHandler) {
    let data = vec![9, 0, 5, 1];

    // test host encrypt, client decrypt
    let host_cipher = host.unreliable_cipher();
    let (ciphertext, counter) = host_cipher.encrypt(&data).unwrap();
    let message = WpskkaMessage::TransportDataMessageUnreliable(TransportDataMessageUnreliable {
        data: ciphertext,
        counter,
    });
    let mut write = Vec::new();
    let mut events = Vec::new();
    let result = client
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .expect("expected data");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);
    assert_eq!(&result, &data);

    // test client encrypt, host decrypt
    let client_cipher = client.unreliable_cipher();
    let (ciphertext, counter) = client_cipher.encrypt(&data).unwrap();
    let message = WpskkaMessage::TransportDataMessageUnreliable(TransportDataMessageUnreliable {
        data: ciphertext,
        counter,
    });
    let mut write = Vec::new();
    let mut events = Vec::new();
    let result = host
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .expect("expected data");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);
    assert_eq!(&result, &data);
}


fn test_communication(host: &mut WpskkaHostHandler, client: &mut WpskkaClientHandler) {
    test_reliable_communication(host, client);
    test_unreliable_communication(host, client);
}

#[test]
fn wpskka_test_srp_static() {
    let mut host = WpskkaHostHandler::new();
    let mut client = WpskkaClientHandler::new();

    let password = b"static".to_vec();

    host.set_static_password(Some(password.clone()));

    srp_authenticate(&mut host, &mut client, &password);

    test_communication(&mut host, &mut client);
}

#[test]
fn wpskka_test_srp_dynamic() {
    let mut host = WpskkaHostHandler::new();
    let mut client = WpskkaClientHandler::new();

    let password = b"dynamic".to_vec();

    host.set_dynamic_password(Some(password.clone()));

    srp_authenticate(&mut host, &mut client, &password);

    test_communication(&mut host, &mut client);
}

// TODO test if we have both static and dynamic
