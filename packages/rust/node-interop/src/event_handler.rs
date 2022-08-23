// when peer emits an event it ends up here for us to handle it or forward it to node via callback_interface

use crate::{forward, instance::Instance};
use peer::{
    rvd::{RvdClientInform, RvdHostInform},
    svsc_handler::SvscInform,
    wpskka::{WpskkaClientInform, WpskkaHostInform},
    InformEvent,
};
use peer_util::{
    decoder::Decoder,
    rvd_native_helper::{rvd_client_native_helper, rvd_host_native_helper},
};
use std::time::UNIX_EPOCH;
use video_process::rtp::RtpDecoder;

pub fn handle_event(instance: &mut Instance, event: InformEvent) -> Result<(), ()> {
    match event {
        InformEvent::SvscInform(event) => match event {
            SvscInform::VersionBad => instance
                .callback_interface
                .svsc_version_bad(&instance.channel),
            SvscInform::LeaseUpdate => {
                let lease_id = forward!(instance.sv_handler, [HostSignal, ClientSignal], |stack| {
                    stack.lease_id()
                });
                instance
                    .callback_interface
                    .svsc_lease_update(&instance.channel, u32::from_be_bytes(lease_id).to_string())
            }
            SvscInform::SessionUpdate => instance
                .callback_interface
                .svsc_session_update(&instance.channel),
            SvscInform::SessionEnd => instance
                .callback_interface
                .svsc_session_end(&instance.channel),
            SvscInform::LeaseRequestRejected => instance
                .callback_interface
                .svsc_session_end(&instance.channel),
            SvscInform::SessionRequestRejected(status) => instance
                .callback_interface
                .svsc_error_session_request_rejected(&instance.channel, status),
            SvscInform::LeaseExtensionRequestRejected => instance
                .callback_interface
                .svsc_error_lease_extension_request_rejected(&instance.channel),
        },
        InformEvent::RvdClientInform(event) => {
            let event = match rvd_client_native_helper(event, &mut instance.native)
                .expect("rvd_client_native_helper failed")
            {
                None => return Ok(()),
                Some(event) => event,
            };

            match event {
                RvdClientInform::HandshakeComplete => {
                    instance
                        .callback_interface
                        .rvd_client_handshake_complete(&instance.channel);
                }
                RvdClientInform::FrameData(data) => {
                    let decoder = instance.decoders.get_mut(&data.display_id).unwrap();
                    let sample = decoder.decode_to_vp9(data.data.0.to_vec());
                    if let Some(sample) = sample {
                        let timestamp = sample
                            .timestamp
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        instance.callback_interface.rvd_client_frame_data(
                            &instance.channel,
                            data.display_id,
                            sample.data.to_vec(),
                            timestamp as _,
                            false,
                        );
                    }
                }
                RvdClientInform::DisplayShare(share) => {
                    instance
                        .decoders
                        .insert(share.display_id, RtpDecoder::new());
                    instance
                        .callback_interface
                        .rvd_client_display_share(&instance.channel, share);
                }
                RvdClientInform::DisplayUnshare(unshare) => {
                    instance.decoders.remove(&unshare);
                    instance
                        .callback_interface
                        .rvd_client_display_unshare(&instance.channel, unshare);
                }
                _ => {}
            }
        }
        InformEvent::RvdHostInform(event) => {
            let (inform, msg) =
                rvd_host_native_helper(event, &mut instance.native, &instance.shared_displays)
                    .expect("rvd_host_native_helper failed");
            if let Some(inform) = inform {
                match inform {
                    RvdHostInform::HandshakeComplete => {
                        instance
                            .callback_interface
                            .rvd_host_handshake_complete(&instance.channel);
                    }
                    _ => {}
                }
            }
            if let Some(_msg) = msg {
                // TODO
            }
        }
        InformEvent::WpskkaClientInform(event) => match event {
            WpskkaClientInform::AuthScheme(auths) => {
                instance.auth_schemes = auths;
                match instance.next_auth_scheme() {
                    Ok(_) => {}
                    Err(_) => {
                        instance
                            .callback_interface
                            .wpskka_client_authentication_failed(&instance.channel);
                    }
                };
            }
            WpskkaClientInform::PasswordPrompt =>
                if let Some(password) = &instance.password {
                    forward!(instance.sv_handler, [ClientSignal, ClientDirect], |stack| {
                        stack.process_password(password.as_bytes())
                    });
                } else {
                    instance
                        .callback_interface
                        .wpskka_client_password_prompt(&instance.channel)
                },
            WpskkaClientInform::AuthFailed => instance
                .callback_interface
                .wpskka_client_authentication_failed(&instance.channel),
            WpskkaClientInform::AuthSuccessful => {
                forward!(instance.sv_handler, [ClientSignal, ClientDirect], |stack| {
                    stack.protocol_version()
                });
                instance
                    .callback_interface
                    .wpskka_client_authentication_successful(&instance.channel)
            }
        },
        InformEvent::WpskkaHostInform(event) => match event {
            WpskkaHostInform::AuthSuccessful => instance
                .callback_interface
                .wpskka_host_authentication_successful(&instance.channel),
            WpskkaHostInform::AuthFailed => {}
        },
    }
    Ok(())
}
