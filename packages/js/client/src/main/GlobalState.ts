import path from 'path';
import { Menu } from 'electron';
import { makeAutoObservable } from 'mobx';
import {
    ClientInstance,
    HostDirectInstance,
    HostInstance,
    HostSignalInstance,
    Instance,
    InstanceConnectionType,
    InstancePeerType,
    JSBox,
} from 'node-interop';
import createMainWindow from './factories/createMainWindow';
import Config from './config';
import BrowserWindow = Electron.BrowserWindow;
import Tray = Electron.Tray;
import MenuItemConstructorOptions = Electron.MenuItemConstructorOptions;
import MenuItem = Electron.MenuItem;
import VTableEmitter from './interopHelpers/VTableEmitter';

interface ClientBundle {
    instance: ClientInstance;
    window: BrowserWindow | null;
    emitter: VTableEmitter;
}

export default class GlobalState {
    mainWindow: BrowserWindow | null = null;

    tray: Tray | null = null;

    signalHostInstance: HostSignalInstance | null = null;

    directHostInstance: HostDirectInstance | null = null;

    clientBundles: ClientBundle[] = [];

    sessionId: string | null = null;

    config: Config;

    constructor(config: Config = new Config()) {
        this.config = config;
        makeAutoObservable(this);
    }
}
