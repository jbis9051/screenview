import path from 'path';
import { Menu } from 'electron';
import { makeAutoObservable } from 'mobx';
import { rust } from 'node-interop';
import Config from './config';
import BrowserWindow = Electron.BrowserWindow;
import Tray = Electron.Tray;
import VTableEmitter from './interopHelpers/VTableEmitter';

export interface ClientBundle {
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
        makeAutoObservable(this);
    }
}
