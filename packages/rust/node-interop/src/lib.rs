#![deny(rust_2018_idioms)]

mod handler;
mod instance;
mod node_interface;
mod protocol;

use common::messages::{
    rvd::ButtonsMask,
    svsc::{Cookie, LeaseId},
};
use instance::*;
use neon::{prelude::*, types::buffer::TypedArray};
use node_interface::NodeInterface;
use num_traits::FromPrimitive;
use protocol::{ConnectionType, Display, DisplayType, Message, RequestContent};
use std::{any::type_name, convert::TryFrom, num::FpCategory};

#[macro_export]
macro_rules! throw {
    ($cx:expr, $error:expr) => {{
        let error = neon::prelude::JsError::error(&mut $cx, $error.to_string());
        error.and_then(|error| $cx.throw(error))
    }};
}

fn integer_arg<T>(cx: &mut FunctionContext<'_>, index: i32) -> NeonResult<T>
where T: FromPrimitive {
    let value = cx.argument::<JsNumber>(index)?.value(cx);
    checked_int_cast(cx, value)
}

fn checked_int_cast<T>(cx: &mut FunctionContext<'_>, value: f64) -> NeonResult<T>
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
fn send_request<'a>(
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

fn new_instance(mut cx: FunctionContext<'_>) -> JsResult<'_, JsBox<InstanceHandle>> {
    let peer_type = cx.argument::<JsString>(0)?.value(&mut cx);
    let instance_type = cx.argument::<JsString>(1)?.value(&mut cx);
    let channel = cx.channel();
    let interface_obj = cx.argument::<JsObject>(2)?;
    let node_interface = NodeInterface::from_obj(&mut cx, interface_obj)?;

    let handle = match (peer_type.as_str(), instance_type.as_str()) {
        ("client", "direct") => Instance::new_client_direct(channel, node_interface),
        ("host", "direct") => Instance::new_host_direct(channel, node_interface),
        ("client", "signal") => Instance::new_client_signal(channel, node_interface),
        ("host", "signal") => Instance::new_host_signal(channel, node_interface),
        _ =>
            if peer_type != "client" && peer_type != "host" {
                return throw!(cx, "Invalid peer type");
            } else {
                return throw!(cx, "Invalid instance type");
            },
    };

    match handle {
        Ok(handle) => Ok(cx.boxed(handle)),
        Err(error) => throw!(cx, error),
    }
}

fn connect(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
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

fn start_server(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let addr = cx.argument::<JsString>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::StartServer { addr })
}

fn establish_session(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
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

fn process_password(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let password = cx.argument::<JsString>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::ProcessPassword {
        password: password.into_bytes(),
    })
}

fn mouse_input(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
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

fn keyboard_input(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let keycode = integer_arg::<u32>(&mut cx, 1)?;
    let down = cx.argument::<JsBoolean>(2)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::KeyboardInput {
        keycode,
        down,
    })
}

fn lease_request(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
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

fn update_static_password(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
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

fn set_controllable(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let is_controllable = cx.argument::<JsBoolean>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::SetControllable {
        is_controllable,
    })
}

fn set_clipboard_readable(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let is_readable = cx.argument::<JsBoolean>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::SetClipboardReadable {
        is_readable,
    })
}

fn share_displays(mut cx: FunctionContext<'_>) -> JsResult<'_, JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let js_displays = cx.argument::<JsArray>(1)?;
    let len = js_displays.len(&mut cx);
    let mut displays = Vec::with_capacity(usize::try_from(len).unwrap());

    for i in 0 .. len {
        let obj = js_displays.get::<JsObject, _, _>(&mut cx, i)?;
        let native_id = obj
            .get::<JsNumber, _, _>(&mut cx, "native_id")?
            .value(&mut cx);
        let display_type = obj.get::<JsString, _, _>(&mut cx, "type")?.value(&mut cx);

        displays.push(Display {
            native_id: checked_int_cast(&mut cx, native_id)?,
            display_type: DisplayType::try_from(display_type.as_str()).unwrap(),
        });
    }

    send_request(&mut cx, handle, RequestContent::ShareDisplays { displays })
}

#[neon::main]
fn main(mut cx: ModuleContext<'_>) -> NeonResult<()> {
    macro_rules! export {
        ( $( $name:ident ),* ) => {
            $(
                cx.export_function(stringify!($name), $name)?;
            )*
        };
    }

    export! {
        new_instance,
        start_server,
        connect,
        establish_session,
        process_password,
        mouse_input,
        keyboard_input,
        lease_request,
        update_static_password,
        set_controllable,
        set_clipboard_readable,
        share_displays
    }

    Ok(())
}
