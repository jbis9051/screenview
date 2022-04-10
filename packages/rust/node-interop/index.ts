export * from './index.node';

export enum InstanceType {
    Host = 'host',
    Client = 'client',
}

export const enum ConnectionType {
    Reliable = 0,
    Unreliable = 1,
}