import { makeAutoObservable } from 'mobx';
import { DisplayInformation } from '@screenview/node-interop';

export enum ConnectionStatus {
    Connecting,
    Handshaking,
    Connected,
    Disconnected,
    Error,
}

export enum ViewMode {
    Grid,
    Single,
}

class UIStore {
    connectionStatus = ConnectionStatus.Connecting;

    error: string | null = null;

    controlling = true;

    controllable = false;

    viewMode = ViewMode.Grid;

    displayInformation: DisplayInformation[] = [];

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UIStore();
