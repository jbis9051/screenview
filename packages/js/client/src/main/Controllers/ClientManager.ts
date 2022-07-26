import { ButtonMask, InstanceConnectionType } from '@screenview/node-interop';
import ClientInstance from './ClientInstance';
import ClientWindow from '../ViewModel/ClientWindow';
import { RendererToMainIPCEvents } from '../../common/IPCEvents';

export default class ClientManager<T extends InstanceConnectionType> {
    window: ClientWindow = new ClientWindow();

    instance: ClientInstance<T>;

    readonly type: T;

    private constructor(type: T, id: string) {
        this.type = type;
        this.instance = new ClientInstance<T>(type, id);
        this.setupListeners();
    }

    static new(id: string): ClientManager<any> {
        throw new Error('Not implemented');
    }

    private setupListeners() {
        this.window.on(
            RendererToMainIPCEvents.Client_PasswordInput,
            (...args: any[]) => {
                // @ts-ignore
                this.instance.processPassword(...args);
            }
        );
        this.window.on(RendererToMainIPCEvents.Client_MouseInput, (...args) => {
            // @ts-ignore
            this.instance.mouseInput(...args);
        });

        this.window.on(
            RendererToMainIPCEvents.Client_KeyboardInput,
            (...args) => {
                // @ts-ignore
                this.instance.keyInput(...args);
            }
        );
    }
}
