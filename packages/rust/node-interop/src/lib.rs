#![deny(rust_2018_idioms)]

mod callback_interface;
mod entrypoints;
mod event_handler;
mod instance;
mod instance_handler;
mod instance_main;
mod protocol;
mod screenview_handler;
mod thumbnail_driver;

use entrypoints::*;
use neon::prelude::*;

#[neon::main]
fn main(mut cx: ModuleContext<'_>) -> NeonResult<()> {
    macro_rules! export {
        ( $( $name:ident ),* ) => {
            $(
                cx.export_function(stringify!($name), $name)?;
            )*
        };
    }

    // these are the functions that node can call
    // changes to the function signatures require an update in index.node.d.ts
    export! {
        new_instance,
        close_instance,
        start_server,
        connect,
        establish_session,
        process_password,
        mouse_input,
        keyboard_input,
        lease_request,
        dangerously_set_no_auth,
        set_controllable,
        set_clipboard_readable,
        share_displays,
        thumbnails,
        close_thumbnails,
        available_displays,
        macos_accessibility_permission,
        macos_screen_capture_permission,
        macos_screen_capture_permission_prompt
    }

    Ok(())
}
