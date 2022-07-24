import { ipcMain } from 'electron';
import events from 'events';
import MainWindow from '../ViewModel/MainWindow';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import Config from '../../common/Config';

export default class MainManager extends events.EventEmitter {
    window: MainWindow | null = null;

    constructor() {
        super();
        this.registerListeners();
    }

    focus() {
        if (!this.window) {
            this.window = new MainWindow();
            this.window.on('close', () => {
                this.window = null;
            });
        } else {
            this.window.focus();
        }
    }

    registerListeners() {
        ipcMain.on(
            RendererToMainIPCEvents.Main_EstablishSession,
            (_, id: string) =>
                this.emit(RendererToMainIPCEvents.Main_EstablishSession, id)
        );
        ipcMain.on(RendererToMainIPCEvents.Main_ConfigRequest, (e) =>
            this.emit(
                RendererToMainIPCEvents.Main_ConfigRequest,
                (config: Config) => {
                    e.sender.send(
                        MainToRendererIPCEvents.Main_ConfigResponse,
                        config
                    );
                }
            )
        );
        ipcMain.on(RendererToMainIPCEvents.Main_ConfigUpdate, (_, config) => {
            this.emit(RendererToMainIPCEvents.Main_ConfigUpdate, config);
        });
    }
}
