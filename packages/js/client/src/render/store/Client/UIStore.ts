import { makeAutoObservable } from 'mobx';
import { NativeThumbnail, Display } from '@screenview/node-interop';

export enum ConnectionStatus {
    Connecting,
    Handshaking,
    Connected,
    Disconnected,
    Error,
}

class UIStore {
    connectionStatus = ConnectionStatus.Connecting;

    error: string | null = null;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UIStore();
