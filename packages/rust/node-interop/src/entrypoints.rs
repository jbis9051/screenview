// these are direct entrypoints to the rust code callable by node
// changes to the function signatures require an update in packages/js/node-interop/index.node.d.ts

use crate::{
    callback_interface::NodeInterface,
    instance_handler::InstanceHandle,
    protocol::{ConnectionType, Message, RequestContent},
    thumbnail_driver::ThumbnailHandle,
};
use common::messages::{
    rvd::ButtonsMask,
    svsc::{Cookie, LeaseId},
};
use native::{
    api::{NativeApiTemplate, NativeId},
    NativeApi,
};
use neon::{prelude::*, types::buffer::TypedArray};
use num_traits::FromPrimitive;
use std::{any::type_name, convert::TryFrom, num::FpCategory};

#[macro_export]
macro_rules! throw {
    ($cx:expr, $error:expr) => {{
        let error = neon::prelude::JsError::error(&mut $cx, $error.to_string());
        error.and_then(|error| $cx.throw(error))
    }};
}

pub fn integer_arg<T>(cx: &mut FunctionContext<'_>, index: i32) -> NeonResult<T>
where T: FromPrimitive {
    let value = cx.argument::<JsNumber>(index)?.value(cx);
    checked_int_cast(cx, value)
}

pub fn checked_int_cast<T>(cx: &mut FunctionContext<'_>, value: f64) -> NeonResult<T>
where T: FromPrimitive {
    if matches!(value.classify(), FpCategory::Infinite | FpCategory::Nan) {
        return throw!(*cx, "Invalid argument: number must not be infinite or NaN");
    }

    if !matches!(value.fract().classify(), FpCategory::Zero) {
        return throw!(*cx, "Invalid argument: number must be an integer");
    }

    match T::from_f64(value) {
        Some(value) => Ok(value),
        None => {
            let msg = format!(
                "Invalid argument: number provided does not fit within type {}",
                type_name::<T>()
            );
            throw!(*cx, msg)
        }
    }
}

// This function is infallible but returns a result for convenience
pub fn send_request<'a>(
    cx: &mut FunctionContext<'a>,
    handle: Handle<'_, JsBox<InstanceHandle>>,
    content: RequestContent,
) -> JsResult<'a, JsPromise> {
    let (deferred, promise) = cx.promise();

    if handle.send(Message::request(content, deferred)) {
        Ok(promise)
    } else {
        panic!("Failed to send node request to handler: broken pipe");
    }
}

pub fn new_instance(mut cx: FunctionContext<'_>) -> JsResult<'_, JsBox<InstanceHandle>> {
    let peer_type = cx.argument::<JsString>(0)?.value(&mut cx);
    let instance_type = cx.argument::<JsString>(1)?.value(&mut cx);
    let channel = cx.channel();
    let interface_obj = cx.argument::<JsObject>(2)?;
    let node_interface = NodeInterface::from_obj(&mut cx, interface_obj)?;

    let handle = match (peer_type.as_str(), instance_type.as_str()) {
        ("client", "direct") => InstanceHandle::new_client_direct(channel, node_interface),
        ("host", "direct") => InstanceHandle::new_host_direct(channel, node_interface),
        ("client", "signal") => InstanceHandle::new_client_signal(channel, node_interface),
        ("host", "signal") => InstanceHandle::new_host_signal(channel, node_interface),
        _ =>
            return if peer_type != "client" && peer_type != "host" {
                throw!(cx, "Invalid peer type")
            } else {
                throw!(cx, "Invalid instance type")
            },
    };
    match handle {
        Ok(handle) => Ok(cx.boxed(handle)),
        Err(error) => throw!(cx, error),
    }
}

pub fn connect(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let connection_type = integer_arg::<u8>(&mut cx, 1)?;

    let connection_type = match ConnectionType::try_from(connection_type as u8) {
        Ok(connection_type) => connection_type,
        Err(_) => return throw!(cx, "Invalid connection type."),
    };
    let addr = cx.argument::<JsString>(2)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::Connect {
        addr,
        connection_type,
    })
}

pub fn start_server(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let addr = cx.argument::<JsString>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::StartServer { addr })
}

pub fn establish_session(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let lease_id: LeaseId = match cx
        .argument::<JsString>(1)?
        .value(&mut cx)
        .as_bytes()
        .try_into()
    {
        Ok(id) => id,
        Err(_) => return throw!(cx, "Lease ID must contain 4 bytes"),
    };

    send_request(&mut cx, handle, RequestContent::EstablishSession {
        lease_id,
    })
}

pub fn process_password(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let password = cx.argument::<JsString>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::ProcessPassword {
        password: password.into_bytes(),
    })
}

pub fn mouse_input(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let x_position = cx.argument::<JsNumber>(1)?.value(&mut cx) as i32;
    let y_position = cx.argument::<JsNumber>(2)?.value(&mut cx) as i32;

    let button_mask = integer_arg::<u8>(&mut cx, 3)?;
    let button_mask_state = integer_arg::<u8>(&mut cx, 4)?;
    let (button_mask, button_mask_state) = match (
        ButtonsMask::from_bits(button_mask),
        ButtonsMask::from_bits(button_mask_state),
    ) {
        (Some(button_mask), Some(button_mask_state)) => (button_mask, button_mask_state),
        _ => return throw!(cx, "Invalid button mask: invalid bit pattern"),
    };

    send_request(&mut cx, handle, RequestContent::MouseInput {
        x_position,
        y_position,
        button_mask,
        button_mask_state,
    })
}

pub fn keyboard_input(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let keycode = integer_arg::<u32>(&mut cx, 1)?;
    let down = cx.argument::<JsBoolean>(2)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::KeyboardInput {
        keycode,
        down,
    })
}

pub fn lease_request(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let cookie: Option<Cookie> = match cx
        .argument::<JsValue>(1)?
        .downcast::<JsArrayBuffer, _>(&mut cx)
    {
        Ok(array_buf) => match array_buf.as_slice(&cx).try_into() {
            Ok(cookie) => Some(cookie),
            Err(_) => return throw!(cx, "Cookie is incorrect length"),
        },
        Err(_) => None,
    };

    send_request(&mut cx, handle, RequestContent::LeaseRequest { cookie })
}

pub fn update_static_password(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let password = cx
        .argument::<JsValue>(1)?
        .downcast::<JsString, _>(&mut cx)
        .ok()
        .map(|string| string.value(&mut cx).into_bytes());

    send_request(&mut cx, handle, RequestContent::UpdateStaticPassword {
        password,
    })
}

pub fn set_controllable(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let is_controllable = cx.argument::<JsBoolean>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::SetControllable {
        is_controllable,
    })
}

pub fn set_clipboard_readable(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let is_readable = cx.argument::<JsBoolean>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::SetClipboardReadable {
        is_readable,
    })
}

pub fn share_displays(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let js_displays = cx.argument::<JsArray>(1)?;
    let controllable = cx.argument::<JsBoolean>(2)?.value(&mut cx);

    let len = js_displays.len(&mut cx);
    let mut displays = Vec::with_capacity(usize::try_from(len).unwrap());

    for i in 0 .. len {
        let obj = js_displays.get::<JsObject, _, _>(&mut cx, i)?;
        let native_id = obj
            .get::<JsNumber, _, _>(&mut cx, "native_id")?
            .value(&mut cx);
        let native_id: u32 = checked_int_cast(&mut cx, native_id)?;
        let display_type = obj.get::<JsString, _, _>(&mut cx, "type")?.value(&mut cx);

        let display = match display_type.as_str() {
            "monitor" => NativeId::Monitor(native_id),
            "window" => NativeId::Window(native_id),
            _ => return throw!(cx, "invalid display type"),
        };

        displays.push(display);
    }

    send_request(&mut cx, handle, RequestContent::ShareDisplays {
        displays,
        controllable,
    })
}


pub fn thumbnails(mut cx: FunctionContext<'_>) -> JsResult<'_, JsBox<ThumbnailHandle>> {
    let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);
    let channel = cx.channel();

    let handle = match ThumbnailHandle::new(channel, callback) {
        Ok(handle) => handle,
        Err(error) => return throw!(cx, format!("{}", error)),
    };

    Ok(cx.boxed(handle))
}

pub fn close_thumbnails(mut cx: FunctionContext<'_>) -> JsResult<'_, JsUndefined> {
    let handle = cx.argument::<JsBox<ThumbnailHandle>>(0)?;
    handle.close();
    Ok(cx.undefined())
}

pub fn available_displays(mut cx: FunctionContext<'_>) -> JsResult<'_, JsArray> {
    let mut native = NativeApi::new().expect("Failed to create native api");
    let monitors = native.monitors().expect("Failed to get monitors");
    let windows = native.windows().expect("Failed to get monitors");


    let mut displays = Vec::with_capacity(monitors.len() + windows.len());

    for monitor in monitors {
        let js_display = JsObject::new(&mut cx);
        let id = JsNumber::new(&mut cx, monitor.id);
        js_display
            .set(&mut cx, "native_id", id)
            .expect("Failed to set native_id");
        let type_str = JsString::new(&mut cx, "monitor");
        js_display
            .set(&mut cx, "type", type_str)
            .expect("Failed to set type");
        displays.push(js_display);
    }

    for window in windows {
        let js_display = JsObject::new(&mut cx);
        let id = JsNumber::new(&mut cx, window.id);
        js_display
            .set(&mut cx, "native_id", id)
            .expect("Failed to set native_id");
        let type_str = JsString::new(&mut cx, "window");
        js_display
            .set(&mut cx, "type", type_str)
            .expect("Failed to set type");
        displays.push(js_display);
    }
    let js_displays = cx.empty_array();

    for (i, display) in displays.into_iter().enumerate() {
        js_displays
            .set(&mut cx, i as u32, display)
            .expect("Failed to push display");
    }

    Ok(js_displays)
}

pub fn macos_accessibility_permission(mut cx: FunctionContext<'_>) -> JsResult<'_, JsBoolean> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            let prompt = cx.argument::<JsBoolean>(0)?.value(&mut cx);
             Ok(cx.boolean(NativeApi::accessibility_permission(prompt)))
        } else {
            panic!("this function is only available on macos");
        }
    }
}

pub fn macos_screen_capture_permission(mut cx: FunctionContext<'_>) -> JsResult<'_, JsBoolean> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            Ok(cx.boolean(NativeApi::screen_capture_permission()))
        } else {
            panic!("this function is only available on macos");
        }
    }
}

pub fn macos_screen_capture_permission_prompt(
    mut cx: FunctionContext<'_>,
) -> JsResult<'_, JsBoolean> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            Ok(cx.boolean(NativeApi::screen_capture_permission_prompt()))
        } else {
            panic!("this function is only available on macos");
        }
    }
}
