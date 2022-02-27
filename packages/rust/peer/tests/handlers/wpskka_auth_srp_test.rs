use common::messages::auth::srp::SrpMessage;
use peer::services::{
    helpers::crypto::keypair,
    wpskka::{
        auth::{
            srp_client::SrpAuthClient,
            srp_host::{SrpAuthHost, SrpHostError},
        },
        WpskkaClientInform,
    },
};

#[test]
fn auth_succeed() {
    let password = b"yellow submraine";
    let host_keys = keypair().unwrap();
    let client_keys = keypair().unwrap();

    let mut host = SrpAuthHost::new(
        host_keys.public_key.clone(),
        client_keys.public_key.as_ref().to_vec(),
    );
    let mut client = SrpAuthClient::new(
        client_keys.public_key,
        host_keys.public_key.as_ref().to_vec(),
    );

    assert!(!host.is_authenticated());
    assert!(!client.is_authenticated());

    // HostHello

    let message = host.init(password);
    assert!(matches!(message, SrpMessage::HostHello(_)));

    let info = client
        .handle(message)
        .expect("handle failed")
        .expect("expected an inform");

    assert!(!client.is_authenticated());
    assert!(matches!(info, WpskkaClientInform::PasswordPrompt));

    // ClientHello

    let message = client.process_password(password).unwrap();
    assert!(!client.is_authenticated());
    assert!(matches!(message, SrpMessage::ClientHello(_)));

    let message = host.handle(message).expect("handle failed");
    assert!(host.is_authenticated());
    assert!(matches!(message, SrpMessage::HostVerify(_)));

    // HostVerify

    assert!(client.handle(message).expect("handle failed").is_none());
    assert!(client.is_authenticated());
}


#[test]
fn auth_failed() {
    let password = b"yellow submraine";
    let host_keys = keypair().unwrap();
    let client_keys = keypair().unwrap();

    let mut host = SrpAuthHost::new(
        host_keys.public_key.clone(),
        client_keys.public_key.as_ref().to_vec(),
    );
    let mut client = SrpAuthClient::new(
        client_keys.public_key,
        host_keys.public_key.as_ref().to_vec(),
    );

    let message = host.init(password);
    client
        .handle(message)
        .expect("handle failed")
        .expect("expected an inform");

    let message = client.process_password(b"badpassword").unwrap();

    let error = host.handle(message).err().unwrap();

    assert!(!host.is_authenticated());
    assert!(matches!(error, SrpHostError::AuthFailed));
}
