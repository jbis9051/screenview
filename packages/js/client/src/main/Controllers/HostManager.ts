import {
    Display,
    InstanceConnectionType,
    VTableEvent,
} from '@screenview/node-interop';
import HostInstance from './HostInstance';
import HostWindow from '../ViewModel/HostWindow';
import IpcListenerService from '../Services/IpcListenerService';
import { RendererToMainIPCEvents } from '../../common/IPCEvents';

export default class HostManager<T extends InstanceConnectionType> {
    window: HostWindow | null = null;

    instance: HostInstance<T>;

    cleanup: Array<() => void> = [];

    private listenerService: IpcListenerService;

    private readonly type: T;

    constructor(
        type: T,
        listenerService: IpcListenerService,
        hostPort?: string
    ) {
        this.type = type;
        this.listenerService = listenerService;
        this.instance = new HostInstance<T>(type, hostPort);
        this.setUpInstanceListeners();
    }

    init() {
        return this.instance.init();
    }

    onDestroy() {
        this.instance.onDestroy();
        this.window?.onDestroy();
        this.cleanup.forEach((cleanup) => cleanup());
    }

    private setUpInstanceListeners() {
        this.instance.vtable.on(
            VTableEvent.WpsskaHostAuthenticationSuccessful,
            () => {
                this.window = new HostWindow(
                    this.type,
                    () => {
                        console.log('HANDLE CLOSE');
                    },
                    this.listenerService
                );
                this.setUpWindowListeners();
            }
        );
    }

    private setUpWindowListeners() {
        this.cleanup.forEach((cleanup) => cleanup());
        this.cleanup = [];

        if (!this.window) {
            throw new Error('Window is null');
        }

        this.cleanup.push(
            this.listenerService.listen(
                RendererToMainIPCEvents.Host_UpdateDesktopList,
                (event, displays: Display[]) => {
                    this.instance.displayShare(displays);
                },
                this.window.window.id
            )
        );
    }
}
