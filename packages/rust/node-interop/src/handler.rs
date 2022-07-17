use common::messages::svsc::LeaseId;
use io::{DirectServer, IoHandle, TcpHandle, UdpHandle};
use peer::{
    higher_handler::HigherHandler,
    lower::{LowerHandlerDirect, LowerHandlerSignal},
    rvd::{RvdClientHandler, RvdHostHandler},
    wpskka::{WpskkaClientHandler, WpskkaHostHandler},
    InformEvent,
};
use peer_util::handler_stack::{HandlerError, HandlerStack};

type HStack<W, R, L> = HandlerStack<HigherHandler<W, R>, L, TcpHandle, UdpHandle>;
type HostSignalStack = HStack<WpskkaHostHandler, RvdHostHandler, LowerHandlerSignal>;
type HostDirectStack = HStack<WpskkaHostHandler, RvdHostHandler, LowerHandlerDirect>;
type ClientSignalStack = HStack<WpskkaClientHandler, RvdClientHandler, LowerHandlerSignal>;
type ClientDirectStack = HStack<WpskkaClientHandler, RvdClientHandler, LowerHandlerDirect>;

pub(crate) fn call_unary<F, T, U>(arg: T, f: F) -> U
where F: FnOnce(T) -> U {
    f(arg)
}

#[macro_export]
macro_rules! __forward_pat {
    (HostDirect, $stack:ident) => {
        $crate::handler::ScreenViewHandler::HostDirect($stack, _)
    };
    ($variant:ident, $stack:ident) => {
        $crate::handler::ScreenViewHandler::$variant($stack)
    };
}

#[macro_export]
macro_rules! __forward_internal {
    ($stack:ident, $handler:expr, [ $( $variant:ident ),* ], $op:expr) => {
        match &mut $handler {
            $(
                $crate::__forward_pat!($variant, $stack)  => $crate::handler::call_unary($stack, $op),
            )*
            #[allow(unreachable_patterns)]
            _ => unreachable!()
        }
    };
}

#[macro_export]
macro_rules! forward {
    ($handler:expr, [ $( $variant:ident ),* ], $op:expr) => {
        $crate::__forward_internal!(stack, $handler, [ $( $variant ),* ], $op)
    };
}

pub enum ScreenViewHandler {
    HostSignal(HostSignalStack),
    HostDirect(HostDirectStack, Option<DirectServer>),
    ClientSignal(ClientSignalStack),
    ClientDirect(ClientDirectStack),
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
        Self::HostDirect(
            HandlerStack::new(
                HigherHandler::<WpskkaHostHandler, _>::new(),
                LowerHandlerDirect::new(),
                IoHandle::new(),
            ),
            None,
        )
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
        forward!(
            *self,
            [HostSignal, HostDirect, ClientSignal, ClientDirect],
            |stack| &mut stack.io_handle
        )
    }

    pub fn handle_next_message(&mut self) -> Option<Result<Vec<InformEvent>, HandlerError>> {
        forward!(
            *self,
            [HostSignal, HostDirect, ClientSignal, ClientDirect],
            |stack| stack.handle_next_message()
        )
    }

    pub fn lease_id(&mut self) -> LeaseId {
        forward!(*self, [HostSignal, ClientSignal], |stack| stack.lease_id())
    }
}
