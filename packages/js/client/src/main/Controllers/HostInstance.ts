import {
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
} from '@screenview/node-interop';

export default class HostInstance<T extends InstanceConnectionType> {
    instance: rust.JSBox<rust.Instance<InstancePeerType.Host, T>>;

    type: T;

    vtable = new VTableEmitter();

    constructor(type: T, hostPort?: string) {
        this.type = type;
        this.instance = rust.new_instance(
            InstancePeerType.Host,
            type,
            this.vtable
        );
        if (type === InstanceConnectionType.Direct) {
            if (!hostPort) {
                throw new Error('Host port is required for direct connections');
            }
            rust.start_server(
                this.instance as any,
                `127.0.0.1:${hostPort}`,
                `127.0.0.1:${hostPort}`
            );
        }
        throw new Error('Not implemented');
    }

    onDestroy() {
        rust.close_instance(this.instance as any);
    }
}
