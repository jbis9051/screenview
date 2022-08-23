// these are the callbacks available to node used for event handling, rust can emit events to node this way
// i got a bit macro happy in this file
use common::messages::{
    rvd::{AccessMask, DisplayShare},
    svsc::EstablishSessionStatus,
};
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
    u32 => |cx, me| Ok(JsNumber::new(cx, me).upcast()),
    usize => |cx, me| Ok(JsNumber::new(cx, me as u32).upcast()),
    String => |cx, me| Ok(JsString::new(cx, me).upcast()),
    EstablishSessionStatus => |cx, me| Ok(JsNumber::new(cx, me as u8).upcast()),
    AccessMask => |cx, me| Ok(JsNumber::new(cx, me.bits()).upcast()),
    DisplayShare => |cx, me| Ok(js_object!(cx, {
        "display_id": me.display_id,
        "access": me.access,
        "name": me.name
    })),
    Vec<u8> => |cx, me| Ok(JsArrayBuffer::external(cx, me).upcast())
);


macro_rules! vtable_methods {
    (
        $(
          $name: ident(
              $($arg: ident: $atype: ty),*
          )
        ),*,
    ) => {
        struct VTable {
            $($name: Root<JsFunction>,)*
        }
        pub struct NodeInterface {
            vtable: Arc<VTable>,
            cb_obj: Arc<Root<JsObject>>
        }

        impl NodeInterface {
            pub fn from_obj<'a, C: Context<'a>>(cx: &mut C, obj: Handle<'_, JsObject>) -> NeonResult<Self> {
                let cb_obj = obj.root(cx);
                $(let $name = obj.get::<JsFunction, _, _>(cx, stringify!($name))?.root(cx);)*

                Ok(Self {
                    vtable: Arc::new(VTable {
                        $($name,)*
                    }),
                    cb_obj: Arc::new(cb_obj)
                })
            }

            $(
                pub fn $name(&self, channel: &Channel $(,$arg: $atype)*){
                    let vtable = Arc::clone(&self.vtable);
                    let cb_obj = self.cb_obj.clone();
                    channel.send(move |mut cx| {
                        let this = cb_obj.to_inner(&mut cx);
                        let func = vtable.$name.to_inner(&mut cx);
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
    svsc_error_lease_extension_request_rejected(),
    /* wpskka - client */
    wpskka_client_password_prompt(),
    wpskka_client_authentication_successful(),
    wpskka_client_authentication_failed(),
    /* wpskka - host */
    wpskka_host_authentication_successful(),
    /* rvd - client */
    rvd_client_frame_data(display_id: u8, data: Vec<u8>, timestamp: u32, key: bool),
    rvd_client_handshake_complete(),
    /* rvd - host */
    rvd_host_handshake_complete(),
    rvd_client_display_share(share: DisplayShare),
    rvd_client_display_unshare(display_id: u8),
);
