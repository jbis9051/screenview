use peer::{
    handler_stack::{HandlerError, HandlerStack},
    higher_handler::HigherHandler,
    io::{IoHandle, TcpHandle, UdpHandle},
    lower::{LowerHandlerDirect, LowerHandlerSignal},
    rvd::{RvdClientHandler, RvdHostHandler},
    wpskka::{WpskkaClientHandler, WpskkaHostHandler},
    InformEvent,
};

type HStack<W, R, L> = HandlerStack<HigherHandler<W, R>, L, TcpHandle, UdpHandle>;

#[inline(always)]
fn call_unary<F, T, U>(arg: T, f: F) -> U
where F: FnOnce(T) -> U {
    f(arg)
}

#[macro_export]
macro_rules! forward {
    ($handler:expr, $closure:expr) => {{
        match &mut $handler {
            ScreenViewHandler::HostSignal(stack) => call_unary(stack, $closure),
            ScreenViewHandler::HostDirect(stack) => call_unary(stack, $closure),
            ScreenViewHandler::ClientSignal(stack) => call_unary(stack, $closure),
            ScreenViewHandler::ClientDirect(stack) => call_unary(stack, $closure),
        }
    }};
}

pub enum ScreenViewHandler {
    HostSignal(HStack<WpskkaHostHandler, RvdHostHandler, LowerHandlerSignal>),
    HostDirect(HStack<WpskkaHostHandler, RvdHostHandler, LowerHandlerDirect>),
    ClientSignal(HStack<WpskkaClientHandler, RvdClientHandler, LowerHandlerSignal>),
    ClientDirect(HStack<WpskkaClientHandler, RvdClientHandler, LowerHandlerDirect>),
}

impl ScreenViewHandler {
    pub fn new_host_signal() -> Self {
        Self::HostSignal(HandlerStack::new(
            HigherHandler::<WpskkaHostHandler, _>::new(),
            LowerHandlerSignal::new(),
            IoHandle::new(),
        ))
    }

    pub fn new_host_direct() -> Self {
        Self::HostDirect(HandlerStack::new(
            HigherHandler::<WpskkaHostHandler, _>::new(),
            LowerHandlerDirect::new(),
            IoHandle::new(),
        ))
    }

    pub fn new_client_signal() -> Self {
        Self::ClientSignal(HandlerStack::new(
            HigherHandler::<WpskkaClientHandler, _>::new(),
            LowerHandlerSignal::new(),
            IoHandle::new(),
        ))
    }

    pub fn new_client_direct() -> Self {
        Self::ClientDirect(HandlerStack::new(
            HigherHandler::<WpskkaClientHandler, _>::new(),
            LowerHandlerDirect::new(),
            IoHandle::new(),
        ))
    }

    pub fn io_handle(&mut self) -> &mut IoHandle<TcpHandle, UdpHandle> {
        forward!(*self, |stack| &mut stack.io_handle)
    }

    pub fn handle_next_message(&mut self) -> Option<Result<Vec<InformEvent>, HandlerError>> {
        forward!(*self, |stack| stack.handle_next_message())
    }
}
