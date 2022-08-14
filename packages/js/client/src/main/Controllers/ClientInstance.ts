import {
    ButtonMask,
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
} from '@screenview/node-interop';

export default class ClientInstance<T extends InstanceConnectionType> {
    instance: rust.JSBox<rust.Instance<InstancePeerType.Client, T>>;

    type: T;

    vtable = new VTableEmitter();

    id: string;

    constructor(type: T, id: string) {
        this.type = type;
        this.id = id;
        this.instance = rust.new_instance(
            InstancePeerType.Client,
            type,
            this.vtable
        );
    }

    async connect() {
        if (this.type === InstanceConnectionType.Direct) {
            await rust.connect(
                this.instance as any,
                ConnectionType.Reliable,
                this.id
            );
            await rust.connect(
                this.instance as any,
                ConnectionType.Unreliable,
                this.id
            );
        }
    }

    processPassword(password: string) {
        rust.process_password(this.instance as any, password); // I really did try to use TypeScript correctly
    }

    mouseInput(
        x: number,
        y: number,
        buttonMask: ButtonMask,
        buttonMaskState: ButtonMask
    ) {
        rust.mouse_input(
            this.instance as any,
            x,
            y,
            buttonMask,
            buttonMaskState
        );
    }

    keyInput(keyCode: number, down: boolean) {
        rust.keyboard_input(this.instance as any, keyCode, down);
    }

    onDestroy() {
        rust.close_instance(this.instance as any);
    }
}
