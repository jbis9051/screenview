use crate::{
    clipboard_type_map::get_native_clipboard,
    network_mouse_button_to_native::network_mouse_button_to_native,
};
use common::messages::rvd::{ButtonsMask, DisplayId, RvdMessage};
use native::api::{NativeApiTemplate, NativeId};
use peer::{
    rvd::{
        RvdClientError,
        RvdClientHandler,
        RvdClientInform,
        RvdHandlerTrait,
        RvdHostError,
        RvdHostHandler,
        RvdHostInform,
    },
    InformEvent,
};
use std::collections::HashMap;

pub fn rvd_client_native_helper<T: NativeApiTemplate>(
    msg: RvdMessage,
    write: &mut Vec<RvdMessage>,
    events: &mut Vec<InformEvent>,
    rvd: &mut RvdClientHandler,
    native: &mut T,
) -> Result<(), ClientError<T>> {
    let mut local_events = Vec::new();
    rvd._handle(msg, write, &mut local_events)?;
    for inform in local_events {
        match &inform {
            InformEvent::RvdClientInform(event) => match event {
                RvdClientInform::ClipboardNotification(data, clip_type) => {
                    native
                        .set_clipboard_content(&get_native_clipboard(clip_type), data)
                        .map_err(ClientError::NativeError)?;
                }
                _ => {
                    events.push(inform);
                }
            },
            _ => panic!("RVD Client gave a bad inform"),
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum ClientError<T: NativeApiTemplate> {
    RvdClientError(RvdClientError),
    NativeError(T::Error),
}

impl<T: NativeApiTemplate> From<RvdClientError> for ClientError<T> {
    fn from(e: RvdClientError) -> Self {
        ClientError::RvdClientError(e)
    }
}

pub fn rvd_host_native_helper<T: NativeApiTemplate>(
    msg: RvdMessage,
    write: &mut Vec<RvdMessage>,
    events: &mut Vec<InformEvent>,
    rvd: &mut RvdHostHandler,
    native: &mut T,
    rvd_native_id_map: &HashMap<DisplayId, NativeId>,
) -> Result<(), HostError<T>> {
    let mut local_events = Vec::new();
    rvd._handle(msg, &mut local_events)?;
    for inform in local_events {
        match &inform {
            InformEvent::RvdHostInform(event) => match event {
                RvdHostInform::MouseInput(event) => {
                    let native_id = rvd_native_id_map.get(&event.display_id).unwrap(); // TODO
                    match native_id {
                        NativeId::Monitor(id) => {
                            native
                                .set_pointer_position_absolute(
                                    event.x_location as u32,
                                    event.y_location as u32,
                                    *id,
                                )
                                .map_err(HostError::NativeError)?;
                        }
                        NativeId::Window(id) => {
                            native
                                .set_pointer_position_relative(
                                    event.x_location as u32,
                                    event.y_location as u32,
                                    *id,
                                )
                                .map_err(HostError::NativeError)?;
                        }
                    }
                    for mask in ButtonsMask::iter() {
                        let window_id = match native_id {
                            NativeId::Monitor(_) => None,
                            NativeId::Window(id) => Some(*id),
                        };
                        if event.button_delta.contains(*mask) {
                            native
                                .toggle_mouse(
                                    network_mouse_button_to_native(mask),
                                    event.button_state.contains(*mask),
                                    window_id,
                                )
                                .map_err(HostError::NativeError)?;
                        }
                    }
                }
                RvdHostInform::KeyboardInput(input) => {
                    native
                        .key_toggle(input.key, input.down)
                        .map_err(HostError::NativeError)?;
                }
                RvdHostInform::ClipboardRequest(is_content, clip_type) => {
                    write.push(<RvdHostHandler as RvdHandlerTrait>::clipboard_data(
                        native
                            .clipboard_content(&get_native_clipboard(clip_type))
                            .map_err(HostError::NativeError)?,
                        *is_content,
                        clip_type.clone(),
                    ));
                }
                RvdHostInform::ClipboardNotification(data, clip_type) => native
                    .set_clipboard_content(&get_native_clipboard(clip_type), data)
                    .map_err(HostError::NativeError)?,
                _ => {
                    events.push(inform);
                }
            },
            _ => panic!("RVD Host gave a bad inform"),
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum HostError<T: NativeApiTemplate> {
    RvdHostError(RvdHostError),
    NativeError(T::Error),
}

impl<T: NativeApiTemplate> From<RvdHostError> for HostError<T> {
    fn from(e: RvdHostError) -> Self {
        HostError::RvdHostError(e)
    }
}
