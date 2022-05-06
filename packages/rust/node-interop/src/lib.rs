mod handler;
mod instance;
mod protocol;

use common::messages::rvd::ButtonsMask;
use instance::*;
use neon::prelude::*;
use num_traits::FromPrimitive;
use protocol::{ConnectionType, Message, RequestContent};
use std::{any::type_name, convert::TryFrom, num::FpCategory};

macro_rules! throw {
    ($cx:expr, $error:expr) => {{
        let error = JsError::error(&mut $cx, $error.to_string())?;
        $cx.throw(error)
    }};
}

fn integer_arg<T>(cx: &mut FunctionContext<'_>, index: i32) -> NeonResult<T>
where T: FromPrimitive {
    let value = cx.argument::<JsNumber>(index)?.value(cx);

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

fn new_instance(mut cx: FunctionContext) -> JsResult<JsBox<InstanceHandle>> {
    let instance_type = cx.argument::<JsString>(0)?;
    let channel = cx.channel();

    // TODO: support other handler stacks
    let handle = match instance_type.value(&mut cx).as_str() {
        "host" => Instance::new_host_direct(channel),
        "client" => Instance::new_client_direct(channel),
        _ => {
            return throw!(
                cx,
                "Invalid instance type, must either be 'host' or 'client'"
            );
        }
    };

    match handle {
        Ok(handle) => Ok(cx.boxed(handle)),
        Err(error) => throw!(cx, error),
    }
}

fn connect(mut cx: FunctionContext) -> JsResult<JsPromise> {
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

fn establish_session(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let lease_id = cx.argument::<JsString>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::EstablishSession {
        lease_id,
    })
}

fn process_password(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let handle = cx.argument::<JsBox<InstanceHandle>>(0)?;
    let password = cx.argument::<JsString>(1)?.value(&mut cx);

    send_request(&mut cx, handle, RequestContent::ProcessPassword {
        password,
    })
}

fn mouse_input(mut cx: FunctionContext) -> JsResult<JsPromise> {
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

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    macro_rules! export {
        ( $( $name:ident ),* ) => {
            $(
                cx.export_function(stringify!($name), $name)?;
            )*
        };
    }

    export! {
        new_instance,
        connect,
        establish_session,
        process_password,
        mouse_input
    }

    Ok(())
}
