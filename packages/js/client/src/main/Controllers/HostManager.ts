import { InstanceConnectionType } from '@screenview/node-interop';
import HostInstance from './HostInstance';
import HostWindow from '../ViewModel/HostWindow';

export default class HostManager<T extends InstanceConnectionType> {
    window: HostWindow | null = null;

    instance: HostInstance<T>;

    constructor(type: T) {
        this.instance = new HostInstance<T>(type);
    }

    cleanup() {
        this.window?.cleanup();
    }
}
