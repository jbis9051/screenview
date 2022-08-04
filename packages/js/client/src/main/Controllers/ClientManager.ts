import { InstanceConnectionType } from '@screenview/node-interop';
import ClientInstance from './ClientInstance';
import ClientWindow from '../ViewModel/ClientWindow';
import { RendererToMainIPCEvents } from '../../common/IPCEvents';
import IpcListenerService from '../Services/IpcListenerService';

export default class ClientManager<T extends InstanceConnectionType> {
    window: ClientWindow;

    instance: ClientInstance<T>;

    readonly type: T;

    private cleanup: Array<() => void> = [];

    private constructor(
        type: T,
        id: string,
        private listenerService: IpcListenerService,
        private onClose: () => void
    ) {
        this.type = type;
        this.instance = new ClientInstance<T>(type, id);
        this.window = new ClientWindow(onClose);
        this.setupListeners();
    }

    static new(
        id: string,
        listenerService: IpcListenerService,
        onClose: () => void
    ): ClientManager<any> {
        // TODO detect signal
        return new ClientManager<InstanceConnectionType.Direct>(
            InstanceConnectionType.Direct,
            id,
            listenerService,
            onClose
        );
    }

    private setupListeners() {
        this.cleanup.push(
            this.listenerService.listen(
                RendererToMainIPCEvents.Client_PasswordInput,
                (data, ...args) => {
                    // @ts-ignore
                    this.instance.processPassword(...args);
                },
                this.window.window.id
            )
        );

        this.cleanup.push(
            this.listenerService.listen(
                RendererToMainIPCEvents.Client_MouseInput,
                (data, ...args) => {
                    // @ts-ignore
                    this.instance.mouseInput(...args);
                },
                this.window.window.id
            )
        );

        this.cleanup.push(
            this.listenerService.listen(
                RendererToMainIPCEvents.Client_KeyboardInput,
                (data, ...args) => {
                    // @ts-ignore
                    this.instance.keyInput(...args);
                },
                this.window.window.id
            )
        );
    }

    private onDestroy() {
        this.cleanup.forEach((fn) => fn());
    }
}
