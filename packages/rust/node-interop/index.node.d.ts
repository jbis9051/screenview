import type {InstanceType, ConnectionType} from "./index";

// this is an opaque type, pointing to an object in rust memory
export type JSBox = {};

export declare function new_instance(type: InstanceType): JSBox;

export declare function connect(handle: JSBox, type: ConnectionType, addr: string): JSBox;