import type {
    InstancePeerType,
    ConnectionType,
    ButtonMask,
    Display,
    EstablishSessionStatus,
    InstanceConnectionType,
    NativeThumbnail,
    DisplayInformation,
} from './index';

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

export declare function close_instance(instance: AnyInstance): void;

export declare function start_server(
    handle: HostDirectInstance,
    reliable_addr: string,
    unreliable_addr: string
): Promise<undefined>;

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

// Calls to this function replace all current displays with the given ones
export declare function share_displays(
    handle: HostInstance,
    displays: Display[],
    controllable: boolean
): Promise<undefined>;

export declare function thumbnails(
    callback: (thumbnails: NativeThumbnail[]) => void
): ThumbnailHandle;

export declare function close_thumbnails(handle: ThumbnailHandle): void;

export declare function available_displays(): Array<Display>;

/* macos only */
export declare function macos_accessibility_permission(
    prompt: boolean
): boolean;

export declare function macos_screen_capture_permission(): boolean;

export declare function macos_screen_capture_permission_prompt(): boolean;

export declare type ThumbnailHandle = JSBox<ThumbnailHandleType>;

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

interface ThumbnailHandleType {
    readonly __type: unique symbol;
}

export interface VTable {
    /* svsc */
    svsc_version_bad(): void;

    svsc_lease_update(session_id: string): void;

    svsc_session_update(): void;

    svsc_session_end(): void;

    svsc_error_lease_request_rejected(): void;

    svsc_error_session_request_rejected(status: EstablishSessionStatus): void;

    svsc_error_lease_extension_request_rejected(): void;

    /* wpskka - client */
    wpskka_client_password_prompt(): void;

    wpskka_client_authentication_successful(): void;

    wpskka_client_authentication_failed(): void;

    /* wpskka - host */
    wpskka_host_authentication_successful(): void;

    /* rvd - client */
    rvd_display_update(
        clipboard_readable: boolean,
        displays: DisplayInformation[]
    ): void;

    rvd_client_handshake_complete(): void;

    rvd_frame_data(display_id: number, data: ArrayBuffer);

    /* rvd - host */
    rvd_host_handshake_complete(): void;
}
