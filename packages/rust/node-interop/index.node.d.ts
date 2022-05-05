import type {InstanceType, ConnectionType, ButtonMask, DisplayType, Display} from "./index";


export declare function new_instance<T extends InstanceType>(type: T): JSBox<Instance<T>>;

export declare function connect<T extends InstanceType>(handle: JSBox<Instance<T>>, type: ConnectionType, addr: string): Promise<undefined>;

export declare function connect_to_host_direct<T extends InstanceType.Client>(handle: JSBox<Instance<T>>, addr: string): Promise<undefined>;
export declare function establish_session<T extends InstanceType.Client>(handle: JSBox<Instance<T>>, lease_id: string): Promise<undefined>;
export declare function process_password<T extends InstanceType.Client>(handle: JSBox<Instance<T>>, password: string): Promise<undefined>;
export declare function mouse_input<T extends InstanceType.Client>(handle: JSBox<Instance<T>>, x_position: number, y_location: number, button_mask: ButtonMask, button_mask_state: ButtonMask): Promise<undefined>;
export declare function keyboard_input<T extends InstanceType.Client>(handle: JSBox<Instance<T>>, key_code: number, down: boolean): Promise<undefined>;


export declare function lease_request<T extends InstanceType.Host>(handle: JSBox<Instance<T>>): Promise<undefined>; // TODO type server instance
export declare function update_static_password<T extends InstanceType.Host>(handle: JSBox<Instance<T>>, password: string | null): Promise<undefined>;
// export declare function preview_displays<T extends InstanceType.Host>(handle: JSBox<Instance<T>>): Promise<{ monitors: [], windows: []}>;
export declare function set_controllable<T extends InstanceType.Host>(handle: JSBox<Instance<T>>, is_controllable: boolean): Promise<undefined>;
export declare function set_clipboard_readable<T extends InstanceType.Host>(handle: JSBox<Instance<T>>, is_readable: boolean): Promise<undefined>;
export declare function share_displays<T extends InstanceType.Host>(handle: JSBox<Instance<T>>, displays: Display[]): Promise<undefined>;

// this is an opaque type, pointing to an object in rust memory
interface JSBox<T> { readonly __type: T; }
interface Instance<T extends InstanceType> { readonly __type: unique symbol; } // Client or Host instance