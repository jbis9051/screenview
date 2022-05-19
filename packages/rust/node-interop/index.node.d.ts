import type {
  InstancePeerType,
  ConnectionType,
  ButtonMask,
  DisplayType,
  Display,
  EstablishSessionStatus,
  InstanceConnectionType,
} from "./index";

export declare type ClientDirectInstance = JSBox<
  Instance<InstancePeerType.Client, InstanceConnectionType.Direct>
>;
export declare type ClientSignalInstance = JSBox<
  Instance<InstancePeerType.Client, InstanceConnectionType.Signal>
>;
export declare type HostDirectInstance = JSBox<
  Instance<InstancePeerType.Host, InstanceConnectionType.Direct>
>;
export declare type HostSignalInstance = JSBox<
  Instance<InstancePeerType.Host, InstanceConnectionType.Signal>
>;

export declare type HostInstance = HostSignalInstance | HostDirectInstance;
export declare type ClientInstance =
  | ClientSignalInstance
  | ClientDirectInstance;

export declare type DirectInstance = HostDirectInstance | ClientDirectInstance;
export declare type SignalInstance = HostSignalInstance | ClientSignalInstance;

export declare type AnyInstance =
  | HostDirectInstance
  | HostSignalInstance
  | ClientDirectInstance
  | ClientSignalInstance;

export declare function new_instance<
  T extends InstancePeerType,
  U extends InstanceConnectionType
>(peer_type: T, instance_type: U, vtable: VTable): JSBox<Instance<T, U>>;

export declare function connect(
  handle: AnyInstance,
  type: ConnectionType,
  addr: string
): Promise<undefined>;

export declare function establish_session(
  handle: ClientSignalInstance,
  lease_id: string
): Promise<undefined>;

export declare function process_password(
  handle: ClientInstance,
  password: string
): Promise<undefined>;

export declare function mouse_input(
  handle: ClientInstance,
  x_position: number,
  y_location: number,
  button_mask: ButtonMask,
  button_mask_state: ButtonMask
): Promise<undefined>;

export declare function keyboard_input(
  handle: ClientInstance,
  key_code: number,
  down: boolean
): Promise<undefined>;

export declare function lease_request(
  handle: HostSignalInstance
): Promise<undefined>;

export declare function update_static_password(
  handle: HostInstance,
  password: string | null
): Promise<undefined>;

// export  declare function preview_displays<T extends InstanceType.Host>(handle: JSBox<Instance<T>>): Promise<{ monitors: [], windows: []}>;
export declare function set_controllable(
  handle: HostInstance,
  is_controllable: boolean
): Promise<undefined>;

export declare function set_clipboard_readable(
  handle: HostInstance,
  is_readable: boolean
): Promise<undefined>;

export declare function share_displays(
  handle: HostInstance,
  displays: Display[]
): Promise<undefined>;

// this is an opaque type, pointing to an object in rust memory
interface JSBox<T> {
  readonly __type: T;
}

interface Instance<
  T extends InstancePeerType,
  U extends InstanceConnectionType
> {
  readonly __type: unique symbol;
  readonly __phantom_1: T;
  readonly __phantom_2: U;
}

export interface VTable {
  /* svsc */
  svsc_version_bad(): void;
  svsc_lease_update(session_id: string): void;
  svsc_session_update(): void;
  svsc_session_end(): void;
  svsc_error_lease_request_rejected(): void;
  svsc_error_session_request_rejected(status: EstablishSessionStatus): void;
  svsc_error_lease_extention_request_rejected(): void;
  /* wpskka - client */
  wpskka_client_password_prompt(): void;
  wpskka_client_authentication_successful(): void;
  wpskka_client_out_of_authentication_schemes(): void;
}
