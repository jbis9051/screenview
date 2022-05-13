use peer::{
    handler_stack::{HandlerError, HandlerStack, HigherStack, LowerStack},
    higher_handler::{HigherHandler, HigherHandlerClient, HigherHandlerHost},
    io::{DirectServer, IoHandle, TcpHandle, UdpHandle},
    lower::{LowerHandlerDirect, LowerHandlerSignal},
    rvd::{RvdClientHandler, RvdHostHandler},
    wpskka::{WpskkaClientHandler, WpskkaHostHandler},
    InformEvent,
};

type HStack<W, R, L> = HandlerStack<HigherHandler<W, R>, L, TcpHandle, UdpHandle>;
type HostSignalStack = HStack<WpskkaHostHandler, RvdHostHandler, LowerHandlerSignal>;
type HostDirectStack = HStack<WpskkaHostHandler, RvdHostHandler, LowerHandlerDirect>;
type ClientSignalStack = HStack<WpskkaClientHandler, RvdClientHandler, LowerHandlerSignal>;
type ClientDirectStack = HStack<WpskkaClientHandler, RvdClientHandler, LowerHandlerDirect>;

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
        match self {
            Self::HostSignal(stack) => &mut stack.io_handle,
            Self::HostDirect(stack, ..) => &mut stack.io_handle,
            Self::ClientSignal(stack) => &mut stack.io_handle,
            Self::ClientDirect(stack) => &mut stack.io_handle,
        }
    }

    pub fn handle_next_message(&mut self) -> Option<Result<Vec<InformEvent>, HandlerError>> {
        match self {
            Self::HostSignal(stack) => stack.handle_next_message(),
            Self::HostDirect(stack, ..) => stack.handle_next_message(),
            Self::ClientSignal(stack) => stack.handle_next_message(),
            Self::ClientDirect(stack) => stack.handle_next_message(),
        }
    }

    pub fn view(&mut self) -> HandlerView<&'_ mut Self> {
        HandlerView(self)
    }
}

pub struct HandlerView<T>(T);

impl<'a> HandlerView<&'a mut ScreenViewHandler> {
    pub fn host(self) -> HandlerView<HostView<'a>> {
        match self.0 {
            ScreenViewHandler::HostSignal(stack) => HandlerView(HostView::Signal(stack)),
            ScreenViewHandler::HostDirect(stack, server) =>
                HandlerView(HostView::Direct(stack, server)),
            _ => panic!("Attempted to take a HostView of a non-host handler"),
        }
    }

    pub fn client(self) -> HandlerView<ClientView<'a>> {
        match self.0 {
            ScreenViewHandler::ClientSignal(stack) => HandlerView(ClientView::Signal(stack)),
            ScreenViewHandler::ClientDirect(stack) => HandlerView(ClientView::Direct(stack)),
            _ => panic!("Attempted to take a ClientView of a non-client handler"),
        }
    }

    pub fn any_higher(self) -> HandlerView<AnyHigher<'a>> {
        HandlerView(AnyHigher(self.0))
    }
}

impl<'a> HandlerView<HostView<'a>> {
    pub fn signal(self) -> &'a mut HostSignalStack {
        match self.0 {
            HostView::Signal(stack) => stack,
            _ => panic!("Attempted to take SignalView of non-signal handler"),
        }
    }

    pub fn direct(self) -> (&'a mut HostDirectStack, &'a mut Option<DirectServer>) {
        match self.0 {
            HostView::Direct(stack, server) => (stack, server),
            _ => panic!("Attempted to take DirectView of non-direct handler"),
        }
    }

    pub fn any_lower(self) -> HigherStack<'a, HigherHandlerHost, TcpHandle, UdpHandle> {
        self.0.higher()
    }
}

impl<'a> HandlerView<ClientView<'a>> {
    pub fn signal(self) -> &'a mut ClientSignalStack {
        match self.0 {
            ClientView::Signal(stack) => stack,
            _ => panic!("Attempted to take SignalView of non-signal handler"),
        }
    }

    pub fn direct(self) -> &'a mut ClientDirectStack {
        match self.0 {
            ClientView::Direct(stack) => stack,
            _ => panic!("Attempted to take DirectView of non-direct handler"),
        }
    }

    pub fn any_lower(self) -> HigherStack<'a, HigherHandlerClient, TcpHandle, UdpHandle> {
        self.0.higher()
    }
}

impl<'a> HandlerView<AnyHigher<'a>> {
    pub fn signal(self) -> LowerStack<'a, LowerHandlerSignal, TcpHandle, UdpHandle> {
        match self.0 .0 {
            ScreenViewHandler::HostSignal(stack) => SignalView::Host(stack).lower(),
            ScreenViewHandler::ClientSignal(stack) => SignalView::Client(stack).lower(),
            _ => panic!("Attempted to take SignalView of non-signal handler"),
        }
    }

    pub fn direct(self) -> LowerStack<'a, LowerHandlerDirect, TcpHandle, UdpHandle> {
        match self.0 .0 {
            ScreenViewHandler::HostDirect(stack, ..) => DirectView::Host(stack).lower(),
            ScreenViewHandler::ClientDirect(stack) => DirectView::Client(stack).lower(),
            _ => panic!("Attempted to take DirectView of non-direct handler"),
        }
    }
}

pub struct AnyHigher<'a>(&'a mut ScreenViewHandler);

pub enum HostView<'a> {
    Signal(&'a mut HostSignalStack),
    Direct(&'a mut HostDirectStack, &'a mut Option<DirectServer>),
}

impl<'a> HostView<'a> {
    pub fn higher(self) -> HigherStack<'a, HigherHandlerHost, TcpHandle, UdpHandle> {
        match self {
            Self::Signal(stack) => stack.higher(),
            Self::Direct(stack, ..) => stack.higher(),
        }
    }
}

pub enum ClientView<'a> {
    Signal(&'a mut ClientSignalStack),
    Direct(&'a mut ClientDirectStack),
}

impl<'a> ClientView<'a> {
    pub fn higher(self) -> HigherStack<'a, HigherHandlerClient, TcpHandle, UdpHandle> {
        match self {
            Self::Signal(stack) => stack.higher(),
            Self::Direct(stack) => stack.higher(),
        }
    }
}

pub enum SignalView<'a> {
    Host(&'a mut HostSignalStack),
    Client(&'a mut ClientSignalStack),
}

impl<'a> SignalView<'a> {
    pub fn lower(self) -> LowerStack<'a, LowerHandlerSignal, TcpHandle, UdpHandle> {
        match self {
            Self::Host(stack) => stack.lower(),
            Self::Client(stack) => stack.lower(),
        }
    }
}

pub enum DirectView<'a> {
    Host(&'a mut HostDirectStack),
    Client(&'a mut ClientDirectStack),
}

impl<'a> DirectView<'a> {
    pub fn lower(self) -> LowerStack<'a, LowerHandlerDirect, TcpHandle, UdpHandle> {
        match self {
            Self::Host(stack) => stack.lower(),
            Self::Client(stack) => stack.lower(),
        }
    }
}
