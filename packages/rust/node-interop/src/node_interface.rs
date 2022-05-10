use neon::{object::Object, prelude::*};
use std::sync::Arc;

struct VTable {
    example_fn: Root<JsFunction>,
    // TODO: add functions here
}

pub struct NodeInterface {
    vtable: Arc<VTable>,
}

impl NodeInterface {
    pub fn from_obj<'a, C: Context<'a>>(cx: &mut C, obj: Handle<'_, JsObject>) -> NeonResult<Self> {
        let example_fn = obj.get::<JsFunction, _, _>(cx, "example_fn")?.root(cx);

        Ok(Self {
            vtable: Arc::new(VTable { example_fn }),
        })
    }

    pub fn example_fn(&self, channel: &Channel, arg1: String, arg2: i32) {
        let vtable = Arc::clone(&self.vtable);

        channel.send(move |mut cx| {
            let func = vtable.example_fn.to_inner(&mut cx);
            let this = cx.null();
            let args = [cx.string(arg1).upcast(), cx.number(arg2).upcast()];
            func.call(&mut cx, this, args).map(|_| ())
        });
    }
}
