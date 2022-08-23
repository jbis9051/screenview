import { makeAutoObservable } from 'mobx';
import { DisplayShare } from '@screenview/node-interop';

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

    displayShares: DisplayShare[] = [];

    decoder: Map<number, VideoDecoder> = new Map();

    canvases: Map<number, HTMLCanvasElement> = new Map();

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UIStore();
