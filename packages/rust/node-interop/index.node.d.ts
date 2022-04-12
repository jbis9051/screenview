import type {InstanceType, ConnectionType} from "./index";


export declare function new_instance<T extends InstanceType>(type: T): JSBox<Instance<T>>;
export declare function connect<T extends InstanceType>(handle: JSBox<Instance<T>>, type: ConnectionType, addr: string): Promise<undefined>;

// this is an opaque type, pointing to an object in rust memory
interface JSBox<T> { readonly __type: T; }
interface Instance<T extends InstanceType> { readonly __type: unique symbol; } // Client or Host instance