use common::messages::svsc::EstablishSessionStatus;
use neon::{object::Object, prelude::*};
use std::sync::Arc;

macro_rules! vtable_arg_map {
    (i32, $cx: ident, $arg: ident) => {
        JsNumber::new(&mut $cx, $arg).upcast()
    };
    (u8, $cx: ident, $arg: ident) => {
        JsNumber::new(&mut $cx, $arg).upcast()
    };
    (String, $cx: ident, $arg: ident) => {
        JsString::new(&mut $cx, $arg).upcast()
    };
    (EstablishSessionStatus, $cx: ident, $arg: ident) => {
        JsNumber::new(&mut $cx, $arg as u8).upcast()
    };
    (VecU8, $cx: ident, $arg: ident) => {
        JsArrayBuffer::external(&mut $cx, $arg).upcast()
    };
}

macro_rules! vtable_methods {
    (
        $(
          $name: ident(
              $($arg: ident: $atype: ident),*
          )
        ),*
    ) => {
        struct VTable {
            $($name: Root<JsFunction>,)*
        }
        pub struct NodeInterface {
            vtable: Arc<VTable>,
        }

        impl NodeInterface {
            pub fn from_obj<'a, C: Context<'a>>(cx: &mut C, obj: Handle<'_, JsObject>) -> NeonResult<Self> {
                $(let $name = obj.get::<JsFunction, _, _>(cx, stringify!($name))?.root(cx);)*

                Ok(Self {
                    vtable: Arc::new(VTable {
                        $($name,)*
                    }),
                })
            }

            $(
                pub fn $name(&self, channel: &Channel $(,$arg: $atype)*){
                    let vtable = Arc::clone(&self.vtable);

                    channel.send(move |mut cx| {
                        let func = vtable.$name.to_inner(&mut cx);
                        let this = cx.null();
                        let args = [$(vtable_arg_map!($atype, cx, $arg),)*];
                        func.call(&mut cx, this, args).map(|_| ())
                    });
                }
            )*
        }
    }
}

type VecU8 = Vec<u8>;

vtable_methods!(
    /* svsc */
    svsc_version_bad(),
    svsc_lease_update(lease_id: String), // Inform doesn't contain a session_id so we have to add
    svsc_session_update(),
    svsc_session_end(),
    svsc_error_lease_request_rejected(),
    svsc_error_session_request_rejected(status: EstablishSessionStatus),
    svsc_error_lease_extention_request_rejected(),
    /* wpskka - client */
    wpskka_client_password_prompt(),
    wpskka_client_authentication_successful(),
    wpskka_client_out_of_authentication_schemes(), // aka authentication_failed
    /* rvd - client */
    rvd_frame_data(display_id: u8, data: VecU8)
);
