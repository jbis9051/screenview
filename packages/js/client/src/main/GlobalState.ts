import { makeAutoObservable, observable, toJS } from 'mobx';
import { rust } from 'node-interop';
import Config from '../common/Config';
import BrowserWindow = Electron.BrowserWindow;
import Tray = Electron.Tray;
import VTableEmitter from './interopHelpers/VTableEmitter';

export interface ClientBundle {
    // Internals aren't tracked by MobX. See below.
    instance: rust.ClientInstance;
    window: BrowserWindow | null;
    emitter: VTableEmitter;
}

export default class GlobalState {
    mainWindow: BrowserWindow | null = null;

    tray: Tray | null = null;

    signalHostInstance: rust.HostSignalInstance | null = null;

    signalHostWindow: BrowserWindow | null = null;

    directHostInstance: rust.HostDirectInstance | null = null;

    directHostWindow: BrowserWindow | null = null;

    clientBundles: ClientBundle[] = [];

    sessionId: string | null = null;

    config: Config;

    constructor(config: Config = new Config()) {
        this.config = config;
        // When we call makeAutoObservable, GlobalState objects are turned into observables with mobx.
        // When we try to pass instances to node, it complains because
        // they aren't the right type. Therefore, we must use observable.ref and observable.shallow.
        // Note: This means that reassignment of internals of ClientBundles aren't tracked
        makeAutoObservable(this, {
            directHostInstance: observable.ref,
            signalHostInstance: observable.ref,
            clientBundles: observable.shallow,
        });
    }
}
