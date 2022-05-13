export * as rust from './index.node';

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
    native_id: number, 
    type: DisplayType
}
