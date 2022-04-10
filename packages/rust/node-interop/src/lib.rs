mod instance;
mod protocol;

use instance::*;
use neon::prelude::*;
use protocol::{ConnectionType, Message, RequestContent};
use std::convert::TryFrom;

macro_rules! throw {
    ($cx:expr, $error:expr) => {{
        let error = JsError::error(&mut $cx, $error.to_string())?;
        $cx.throw(error)
    }};
}

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

    let handle = match instance_type.value(&mut cx).as_str() {
        "host" => Instance::new_host(channel),
        "client" => Instance::new_client(channel),
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

    let connection_type = cx.argument::<JsNumber>(1)?.value(&mut cx);
    if connection_type.fract().abs() != 0.0 {
        return throw!(cx, "Connection type must be an integer.");
    }

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

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    macro_rules! export {
        ($name:ident) => {
            cx.export_function(stringify!($name), $name)?
        };
    }

    export!(new_instance);
    export!(connect);
    Ok(())
}
