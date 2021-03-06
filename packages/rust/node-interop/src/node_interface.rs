// i got a bit macro happy in this file
use common::messages::{rvd::AccessMask, svsc::EstablishSessionStatus};
use neon::{object::Object, prelude::*};
use std::sync::Arc;

trait ToJsType {
    fn try_into_js_type<'a, C: Context<'a>>(self, cx: &mut C) -> NeonResult<Handle<'a, JsValue>>;
}

macro_rules! js_object {
    ($cx:ident,
        {
            $(
                $key:tt : $value:expr
            ),*
        }
    ) => {
        {
            let obj = JsObject::new($cx);
            $(
                let value = $value.try_into_js_type($cx)?;
                obj.set($cx, $key, value)?;
            )*
            obj.upcast()
        }
    }
}

macro_rules! js_array {
    ($cx:ident, $vec: ident) => {{
        let arr = JsArray::new($cx, $vec.len() as u32);
        for (i, item) in $vec.into_iter().enumerate() {
            let item = item.try_into_js_type($cx)?;
            arr.set($cx, i as u32, item)?;
        }
        arr.upcast()
    }};
}

macro_rules! impl_try_into_js_type {
    (
        $(
            $atype:ty => |$cx:ident, $arg:ident| $code:expr
        ),*
    ) => {
        $(
            impl ToJsType for $atype {
                fn try_into_js_type<'a, C: Context<'a>>(self, $cx: &mut C) -> NeonResult<Handle<'a, JsValue>> {
                    let $arg = self;
                    $code
                }
            }
        )*
    }
}

impl_try_into_js_type!(
    bool => |cx, me| Ok(JsBoolean::new(cx, me).upcast()),
    i32 => |cx, me| Ok(JsNumber::new(cx, me).upcast()),
    u8 => |cx, me| Ok(JsNumber::new(cx, me).upcast()),
    u16 => |cx, me| Ok(JsNumber::new(cx, me).upcast()),
    String => |cx, me| Ok(JsString::new(cx, me).upcast()),
    EstablishSessionStatus => |cx, me| Ok(JsNumber::new(cx, me as u8).upcast()),
    /*DisplayInformation => |cx, me| Ok(js_object!(cx,
        {
            "display_id" : me.display_id,
            "width" : me.width,
            "height" : me.height,
            "controllable": me.access
                    .contains(AccessMask::CONTROLLABLE)
        }
    )),*/
    Vec<u8> => |cx, me| Ok(JsArrayBuffer::external(cx, me).upcast())
    //Vec<DisplayInformation> => |cx, me| Ok(js_array!(cx, me))
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
                        let args = [$($arg.try_into_js_type(&mut cx)?,)*];
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
    //rvd_display_update(clipboardReadable: bool, displays: Vec<DisplayInformation>),
    rvd_frame_data(display_id: u8, data: Vec<u8>)
);
