export * as rust from './index.node';

export * from './VTableEmitter';

export enum InstancePeerType {
    Host = 'host',
    Client = 'client',
}

export enum InstanceConnectionType {
    Direct = 'direct',
    Signal = 'signal',
}

export enum ConnectionType {
    Reliable = 0,
    Unreliable = 1,
}

export enum ButtonMask {
    LEFT = 1 << 0,
    MIDDLE = 1 << 1,
    RIGHT = 1 << 2,
    SCROLL_UP = 1 << 3,
    SCROLL_DOWN = 1 << 4,
    SCROLL_LEFT = 1 << 5,
    SCROLL_RIGHT = 1 << 6,
}

export enum DisplayType {
    Monitor = 'monitor',
    Window = 'window',
}

export interface Display {
    native_id: number;
    type: DisplayType; // TODO consistent with NativeThumbnail
}

export enum EstablishSessionStatus {
    Success = 0x00,
    IDNotFound = 0x01,
    PeerOffline = 0x02,
    PeerBusy = 0x03,
    SelfBusy = 0x04,
    OtherError = 0x05,
}

export interface NativeThumbnail {
    data: ArrayBuffer;
    name: string;
    native_id: number;
    display_type: DisplayType;
}

export interface DisplayInformation {
    native_id: number;
    name: string;
    width: number;
    height: number;
}
