import {
    ButtonMask,
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
} from '@screenview/node-interop';
import events from 'events';

export default class ClientInstance<
    T extends InstanceConnectionType
> extends events.EventEmitter {
    instance: rust.JSBox<rust.Instance<InstancePeerType.Client, T>>;

    type: T;

    vtable = new VTableEmitter();

    constructor(type: T, id: string) {
        super();
        this.type = type;
        this.instance = rust.new_instance(
            InstancePeerType.Client,
            type,
            this.vtable
        );
    }

    getVtable() {
        return this.vtable;
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
}
