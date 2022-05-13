use neon::{object::Object, prelude::*};
use std::sync::Arc;

macro_rules! vtable_arg_map {
    (number) => {
        i32
    };
    (string) => {
        String
    };
}

macro_rules! vtable_methods {
    (
        $(
          $name: ident => (
              $($arg: ident => $jstype: ident),*
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
                pub fn $name(&self, channel: &Channel $(,$arg: vtable_arg_map!($jstype))*){
                    let vtable = Arc::clone(&self.vtable);

                    channel.send(move |mut cx| {
                        let func = vtable.$name.to_inner(&mut cx);
                        let this = cx.null();
                        let args = [$(cx.$jstype($arg).upcast(),)*];
                        func.call(&mut cx, this, args).map(|_| ())
                    });
                }
            )*
        }
    }
}


vtable_methods!(
    example_fn => (arg1 => number, arg2 => string)
);
