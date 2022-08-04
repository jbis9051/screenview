import { InstanceConnectionType } from '@screenview/node-interop';
import HostInstance from './HostInstance';
import HostWindow from '../ViewModel/HostWindow';

export default class HostManager<T extends InstanceConnectionType> {
    window: HostWindow | null = null;

    instance: HostInstance<T>;

    constructor(type: T, hostPort?: string) {
        this.instance = new HostInstance<T>(type, hostPort);
    }

    cleanup() {
        this.instance.onDestroy();
        this.window?.onDestroy();
    }
}
