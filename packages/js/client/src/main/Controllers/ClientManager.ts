import { InstanceConnectionType, VTableEvent } from '@screenview/node-interop';
import ClientInstance from './ClientInstance';
import ClientWindow from '../ViewModel/ClientWindow';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import IpcListenerService from '../Services/IpcListenerService';
import waitForReadySignal from '../helpers/waitForReadySignal';

export default class ClientManager<T extends InstanceConnectionType> {
    window: ClientWindow;

    instance: ClientInstance<T>;

    readonly type: T;

    private cleanup: Array<() => void> = [];

    ready: Promise<void>;

    private constructor(
        type: T,
        instance: ClientInstance<T>,
        private listenerService: IpcListenerService,
        private onClose: () => void
    ) {
        this.type = type;
        this.instance = instance;
        this.window = new ClientWindow(onClose);
        this.ready = waitForReadySignal(this.window.window);
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
        const instance = new ClientInstance(type, id);
        const manager = new ClientManager(
            type,
            instance,
            listenerService,
            onClose
        );
        await manager.ready;
        await instance.connect();
        return manager;
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
            VTableEvent.RvdClientHandshakeComplete,
            VTableEvent.RvdDisplayShare,
            VTableEvent.RvdDisplayUnshare,
            VTableEvent.RvdClientFrameData,
        ];
        eventsToForward.forEach((event) => {
            this.instance.vtable.on(event, async (...args) => {
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
