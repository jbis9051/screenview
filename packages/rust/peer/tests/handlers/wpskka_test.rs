use common::messages::{
    wpskka::{
        AuthSchemeType,
        TransportDataMessageReliable,
        TransportDataMessageUnreliable,
        WpskkaMessage,
    },
    Data,
};
use peer::{
    wpskka::{
        WpskkaClientHandler,
        WpskkaClientInform,
        WpskkaHandlerTrait,
        WpskkaHostHandler,
        WpskkaHostInform,
    },
    InformEvent,
};
use std::borrow::Cow;

fn key_exchange<'a>(
    host: &mut WpskkaHostHandler,
    client: &mut WpskkaClientHandler,
    write: &mut Vec<WpskkaMessage<'a>>,
    events: &mut Vec<InformEvent>,
) -> WpskkaMessage<'a> {
    let key_exchange = host.key_exchange().expect("host key exchange failed");

    assert!(matches!(key_exchange, WpskkaMessage::KeyExchange(_)));

    let result = client
        .handle(key_exchange, write, events)
        .expect("host->client key exchange failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 1);
    assert!(matches!(write[0], WpskkaMessage::KeyExchange(_)));
    assert_eq!(events.len(), 0);

    let key_exchange = write.remove(0);

    let result = host
        .handle(key_exchange, write, events)
        .expect("client->host key exchange failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 1);
    assert!(matches!(write[0], WpskkaMessage::AuthScheme(_)));
    assert_eq!(events.len(), 0);

    write.remove(0)
}

fn auth_schemes_check(
    client: &mut WpskkaClientHandler,
    write: &mut Vec<WpskkaMessage>,
    events: &mut Vec<InformEvent>,
    auth_schemes: WpskkaMessage,
    check_auth_scheme_types: Option<&[AuthSchemeType]>,
) {
    let result = client
        .handle(auth_schemes, write, events)
        .expect("host->client auth scheme failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let evt = match &events[0] {
        InformEvent::WpskkaClientInform(evt) => evt,
        _ => panic!("expected WpskkaClientInform"),
    };

    let schemes = match evt {
        WpskkaClientInform::AuthScheme(schemes) => schemes,
        _ => panic!("expected AuthScheme scheme"),
    };

    if let Some(check_auth_scheme_types) = check_auth_scheme_types {
        assert_eq!(schemes.len(), check_auth_scheme_types.len());

        for s in check_auth_scheme_types {
            schemes
                .iter()
                .find(|s2| &s == s2)
                .expect("auth scheme not found");
        }
    }

    events.clear();
}

fn test_srp(
    host: &mut WpskkaHostHandler,
    client: &mut WpskkaClientHandler,
    write: &mut Vec<WpskkaMessage>,
    events: &mut Vec<InformEvent>,
    password: &[u8],
    srp_type: AuthSchemeType,
) {
    let auth_schemes = key_exchange(host, client, write, events);

    auth_schemes_check(client, write, events, auth_schemes, Some(&[srp_type]));

    let try_auth = client.try_auth(srp_type);

    let result = host
        .handle(try_auth, write, events)
        .expect("client->host try auth failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 0);

    let auth_msg = write.remove(0);

    let result = client
        .handle(auth_msg, write, events)
        .expect("host->client auth msg failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let evt = match events.remove(0) {
        InformEvent::WpskkaClientInform(evt) => evt,
        _ => panic!("expected WpskkaClientInform"),
    };

    assert!(matches!(evt, WpskkaClientInform::PasswordPrompt));

    let auth_msg = client
        .process_password(password)
        .expect("password prompt failed");

    let result = host
        .handle(auth_msg, write, events)
        .expect("client->host auth msg failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 2);
    assert_eq!(events.len(), 1);

    let evt = match events.remove(0) {
        InformEvent::WpskkaHostInform(evt) => evt,
        _ => panic!("expected WpskkaHostInform"),
    };

    assert!(matches!(evt, WpskkaHostInform::AuthSuccessful));

    let auth_success = write.remove(1);
    let auth_msg = write.remove(0);

    // handle auth_success, client should ignore

    let result = client
        .handle(auth_success, write, events)
        .expect("host->client auth success failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);


    // handle auth_msg

    let result = client
        .handle(auth_msg, write, events)
        .expect("host->client auth success failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let evt = match events.remove(0) {
        InformEvent::WpskkaClientInform(evt) => evt,
        _ => panic!("expected WpskkaClientInform"),
    };

    assert!(matches!(evt, WpskkaClientInform::AuthSuccessful));
}

fn none_auth(
    host: &mut WpskkaHostHandler,
    client: &mut WpskkaClientHandler,
    write: &mut Vec<WpskkaMessage>,
    events: &mut Vec<InformEvent>,
) {
    let auth_schemes = key_exchange(host, client, write, events);

    auth_schemes_check(
        client,
        write,
        events,
        auth_schemes,
        Some(&[AuthSchemeType::None]),
    );

    let try_auth = client.try_auth(AuthSchemeType::None);

    let result = host
        .handle(try_auth, write, events)
        .expect("client->host try auth failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 1);

    let evt = match events.remove(0) {
        InformEvent::WpskkaHostInform(evt) => evt,
        _ => panic!("expected WpskkaHostInform"),
    };

    assert!(matches!(evt, WpskkaHostInform::AuthSuccessful));

    let auth_success = write.remove(0);

    let result = client
        .handle(auth_success, write, events)
        .expect("host->client auth success failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let evt = match events.remove(0) {
        InformEvent::WpskkaClientInform(evt) => evt,
        _ => panic!("expected WpskkaClientInform"),
    };

    assert!(matches!(evt, WpskkaClientInform::AuthSuccessful));
}

#[test]
fn wpskka_test_auth_none() {
    let mut host = WpskkaHostHandler::new();
    let mut client = WpskkaClientHandler::new();
    let mut write = Vec::new();
    let mut events = Vec::new();

    host.set_none_scheme(true);

    none_auth(&mut host, &mut client, &mut write, &mut events);
}


#[test]
fn wpskka_test_auth_none_off_by_default() {
    let mut host = WpskkaHostHandler::new();
    let mut client = WpskkaClientHandler::new();
    let mut write = Vec::new();
    let mut events = Vec::new();

    let auth_schemes = key_exchange(&mut host, &mut client, &mut write, &mut events);

    auth_schemes_check(&mut client, &mut write, &mut events, auth_schemes, None);

    let try_auth = client.try_auth(AuthSchemeType::None);

    let result = host
        .handle(try_auth, &mut write, &mut events)
        .expect("client->host try auth failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 1);

    let evt = match events.remove(0) {
        InformEvent::WpskkaHostInform(evt) => evt,
        _ => panic!("expected WpskkaHostInform"),
    };

    assert!(matches!(evt, WpskkaHostInform::AuthFailed));

    let auth_failed = write.remove(0);

    let result = client
        .handle(auth_failed, &mut write, &mut events)
        .expect("host->client auth success failed");

    assert_eq!(result, None);
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let evt = match events.remove(0) {
        InformEvent::WpskkaClientInform(evt) => evt,
        _ => panic!("expected WpskkaClientInform"),
    };

    assert!(matches!(evt, WpskkaClientInform::AuthFailed));
}

#[test]
fn wpskka_test_auth_srp_static() {
    let mut host = WpskkaHostHandler::new();
    let mut client = WpskkaClientHandler::new();
    let mut write = Vec::new();
    let mut events = Vec::new();

    let password = b"static";

    host.set_static_password(Some(password.to_vec()));

    test_srp(
        &mut host,
        &mut client,
        &mut write,
        &mut events,
        password,
        AuthSchemeType::SrpStatic,
    );
}


#[test]
fn wpskka_test_auth_srp_dynamic() {
    let mut host = WpskkaHostHandler::new();
    let mut client = WpskkaClientHandler::new();
    let mut write = Vec::new();
    let mut events = Vec::new();

    let password = b"dynamic";

    host.set_dynamic_password(Some(password.to_vec()));

    test_srp(
        &mut host,
        &mut client,
        &mut write,
        &mut events,
        password,
        AuthSchemeType::SrpDynamic,
    );
}


#[test]
fn wpskka_test_auth_reliable() {
    let mut host = WpskkaHostHandler::new();
    let mut client = WpskkaClientHandler::new();
    let mut write = Vec::new();
    let mut events = Vec::new();


    host.set_none_scheme(true);

    none_auth(&mut host, &mut client, &mut write, &mut events);

    let data = vec![9, 0, 5, 1];

    // test host encrypt, client decrypt
    let host_cipher = host.reliable_cipher_mut();

    let message = WpskkaMessage::TransportDataMessageReliable(TransportDataMessageReliable {
        data: Data(Cow::Owned(host_cipher.encrypt(&data).unwrap())),
    });

    println!("{:?}", message);

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
        data: Data(Cow::Owned(client_cipher.encrypt(&data).unwrap())),
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


#[test]
fn wpskka_test_auth_unreliable() {
    let mut host = WpskkaHostHandler::new();
    let mut client = WpskkaClientHandler::new();
    let mut write = Vec::new();
    let mut events = Vec::new();


    host.set_none_scheme(true);

    none_auth(&mut host, &mut client, &mut write, &mut events);

    let data = vec![9, 0, 5, 1];

    // test host encrypt, client decrypt
    let host_cipher = host.unreliable_cipher();
    let (ciphertext, counter) = host_cipher.encrypt(&data).unwrap();

    let message = WpskkaMessage::TransportDataMessageUnreliable(TransportDataMessageUnreliable {
        data: Data(Cow::Owned(ciphertext)),
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
        data: Data(Cow::Owned(ciphertext)),
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
