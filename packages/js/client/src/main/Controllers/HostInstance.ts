import {
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
} from '@screenview/node-interop';
import events from 'events';

export default class HostInstance<
    T extends InstanceConnectionType
> extends events.EventEmitter {
    instance: rust.JSBox<rust.Instance<InstancePeerType.Host, T>>;

    type: T;

    vtable = new VTableEmitter();

    constructor(type: T) {
        super();
        this.type = type;
        this.instance = rust.new_instance(
            InstancePeerType.Host,
            type,
            this.vtable
        );
    }

    getVtable() {
        return this.vtable;
    }

    cleanup() {
        rust.close_instance(this.instance as any);
    }
}
