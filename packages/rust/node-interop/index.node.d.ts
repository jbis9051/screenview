import type {InstancePeerType, ConnectionType, ButtonMask, DisplayType, Display} from "./index";
import {InstanceConnectionType} from "./index";


declare type ClientDirectInstance = Instance<InstancePeerType.Client, InstanceConnectionType.Direct>;
declare type ClientSignalInstance = Instance<InstancePeerType.Client, InstanceConnectionType.Signal>;
declare type HostDirectInstance = Instance<InstancePeerType.Host, InstanceConnectionType.Direct>;
declare type HostSignalInstance = Instance<InstancePeerType.Host, InstanceConnectionType.Signal>;

declare type HostInstance = Instance<InstancePeerType.Host, InstanceConnectionType.Direct | InstanceConnectionType.Signal>;
declare type ClientInstance = Instance<InstancePeerType.Client, InstanceConnectionType.Direct | InstanceConnectionType.Signal>;

declare type DirectInstance = Instance<InstancePeerType.Host | InstancePeerType.Client, InstanceConnectionType.Direct>;
declare type SignalInstance = Instance<InstancePeerType.Host | InstancePeerType.Client, InstanceConnectionType.Signal>;

declare type AnyInstance = HostDirectInstance | HostSignalInstance | ClientDirectInstance | ClientSignalInstance;

export declare function new_instance<T extends InstancePeerType, U extends InstanceConnectionType>(peer_type: T, instance_type: U): JSBox<Instance<T, U>>;

export declare function connect(handle: JSBox<ClientInstance>, type: ConnectionType, addr: string): Promise<undefined>;

export declare function connect_to_host_direct(handle: JSBox<ClientDirectInstance>, addr: string): Promise<undefined>;
export declare function establish_session(handle: JSBox<ClientSignalInstance>, lease_id: string): Promise<undefined>;
export declare function process_password(handle: JSBox<ClientInstance>, password: string): Promise<undefined>;
export declare function mouse_input(handle: JSBox<ClientInstance>, x_position: number, y_location: number, button_mask: ButtonMask, button_mask_state: ButtonMask): Promise<undefined>;
export declare function keyboard_input(handle: JSBox<ClientInstance>, key_code: number, down: boolean): Promise<undefined>;


export declare function lease_request(handle: JSBox<HostSignalInstance>): Promise<undefined>; // TODO type server instance
export declare function update_static_password(handle: JSBox<HostInstance>, password: string | null): Promise<undefined>;
// export declare function preview_displays<T extends InstanceType.Host>(handle: JSBox<Instance<T>>): Promise<{ monitors: [], windows: []}>;
export declare function set_controllable(handle: JSBox<HostInstance>, is_controllable: boolean): Promise<undefined>;
export declare function set_clipboard_readable(handle: JSBox<HostInstance>, is_readable: boolean): Promise<undefined>;
export declare function share_displays(handle: JSBox<HostInstance>, displays: Display[]): Promise<undefined>;

// this is an opaque type, pointing to an object in rust memory
interface JSBox<T> { readonly __type: T; }
interface Instance<T extends InstancePeerType, U extends InstanceConnectionType> { readonly __type: unique symbol; readonly __phantom_1: T, readonly __phantom_2: U} // Client or Host instance