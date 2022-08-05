import { InstanceConnectionType, VTableEvent } from '@screenview/node-interop';
import ClientInstance from './ClientInstance';
import ClientWindow from '../ViewModel/ClientWindow';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import IpcListenerService from '../Services/IpcListenerService';

export default class ClientManager<T extends InstanceConnectionType> {
    window: ClientWindow;

    instance: ClientInstance<T>;

    readonly type: T;

    private cleanup: Array<() => void> = [];

    private constructor(
        type: T,
        instance: ClientInstance<T>,
        private listenerService: IpcListenerService,
        private onClose: () => void
    ) {
        this.type = type;
        this.instance = instance;
        this.window = new ClientWindow(onClose);
        this.cleanup.push(() => {
            this.window.onDestroy();
            this.instance.onDestroy();
        });
        this.setupListeners();
    }

    static async new(
        id: string,
        listenerService: IpcListenerService,
        onClose: () => void
    ): Promise<ClientManager<any>> {
        const type = InstanceConnectionType.Direct;
        const instance = await ClientInstance.new(type, id);
        return new ClientManager(type, instance, listenerService, onClose);
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

        const eventsToForward = [
            VTableEvent.WpsskaClientAuthenticationSuccessful,
        ];
        eventsToForward.forEach((event) => {
            this.instance.vtable.on(event, (...args) => {
                this.window.window.webContents.send(
                    MainToRendererIPCEvents.Client_VTableEvent,
                    event,
                    ...args
                );
            });
        });
    }

    onDestroy() {
        this.cleanup.forEach((fn) => fn());
    }
}
