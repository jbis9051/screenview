import {
    Display,
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
} from '@screenview/node-interop';

export default class HostInstance<T extends InstanceConnectionType> {
    instance: rust.JSBox<rust.Instance<InstancePeerType.Host, T>>;

    type: T;

    vtable = new VTableEmitter();

    private readonly hostPort?: string;

    constructor(type: T, hostPort?: string) {
        this.type = type;
        this.instance = rust.new_instance(
            InstancePeerType.Host,
            type,
            this.vtable
        );
        this.hostPort = hostPort;
        // throw new Error('Not implemented');
    }

    init() {
        if (this.type === InstanceConnectionType.Direct) {
            if (!this.hostPort) {
                throw new Error('Host port is required for direct connections');
            }
            rust.start_server(
                this.instance as any,
                `127.0.0.1:${this.hostPort}`,
                `127.0.0.1:${this.hostPort}`
            );
            rust.dangerously_set_no_auth(this.instance as any, true);
        }
    }

    onDestroy() {
        rust.close_instance(this.instance as any);
    }

    displayShare(displays: Display[]) {
        rust.share_displays(this.instance as any, displays, false);
    }
}
