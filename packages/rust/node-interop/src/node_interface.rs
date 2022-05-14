use neon::{object::Object, prelude::*};
use std::sync::Arc;

macro_rules! vtable_arg_map {
    (i32) => {
        JsNumber
    };
    (String) => {
        JsString
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
                        let args = [$(<vtable_arg_map!($atype)>::new(&mut cx, $arg).upcast(),)*];
                        func.call(&mut cx, this, args).map(|_| ())
                    });
                }
            )*
        }
    }
}


vtable_methods!(session_id_update(session_id: String), session_update());
