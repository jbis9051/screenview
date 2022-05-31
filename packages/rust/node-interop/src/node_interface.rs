use common::messages::svsc::EstablishSessionStatus;
use neon::{object::Object, prelude::*};
use std::sync::Arc;

trait ToJsType {
    fn to_js_type<'a, C: Context<'a>>(self, cx: &mut C) -> Handle<'a, JsValue>;
}

macro_rules! impl_to_js_type {
    (
        $(
            $atype:ty => |$cx:ident, $arg:ident| $code:expr
        ),*
    ) => {
        $(
            impl ToJsType for $atype {
                fn to_js_type<'a, C: Context<'a>>(self, $cx: &mut C) -> Handle<'a, JsValue> {
                    let $arg = self;
                    $code
                }
            }
        )*
    }
}

impl_to_js_type!(
    i32 => |cx, me| JsNumber::new(cx, me).upcast(),
    u8 => |cx, me| JsNumber::new(cx, me).upcast(),
    String => |cx, me| JsString::new(cx, me).upcast(),
    EstablishSessionStatus => |cx, me| JsNumber::new(cx, me as u8).upcast(),
    Vec<u8> => |cx, me| JsArrayBuffer::external(cx, me).upcast()
);

macro_rules! vtable_methods {
    (
        $(
          $name: ident(
              $($arg: ident: $atype: ty),*
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
                        let args = [$($arg.to_js_type(&mut cx),)*];
                        func.call(&mut cx, this, args).map(|_| ())
                    });
                }
            )*
        }
    }
}

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
    rvd_frame_data(display_id: u8, data: Vec<u8>)
);
