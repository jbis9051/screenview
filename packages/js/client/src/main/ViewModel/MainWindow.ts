import events from 'events';
import { shell, BrowserWindow } from 'electron';
import { MainToRendererIPCEvents } from '../../common/IPCEvents';
import { MainHeight, MainWidth } from '../../common/contants';
import PageType from '../../render/Pages/PageType';

export default class MainWindow extends events.EventEmitter {
    window: BrowserWindow;

    constructor() {
        super();
        this.window = MainWindow.createWindow();
        this.window.on('close', () => {
            this.emit('close');
        });
    }

    updateSessionId(sessionId: string) {
        this.window.webContents.send(
            MainToRendererIPCEvents.Main_SessionId,
            sessionId
        );
    }

    private static createWindow(): BrowserWindow {
        const mainWindow = new BrowserWindow({
            height: MainHeight,
            width: MainWidth,
            minHeight: MainHeight,
            minWidth: MainWidth,
            titleBarStyle: 'hidden',
            webPreferences: {
                nodeIntegration: true, // I know this is bad but I don't care. We aren't loading third party pages.
                contextIsolation: false,
            },
        });

        mainWindow.webContents.addListener('new-window', (e, url) => {
            e.preventDefault();
            return shell.openExternal(url);
        });

        if (process.env.NODE_ENV === 'development') {
            mainWindow
                .loadURL(`http://localhost:8080/#${PageType.Main}`)
                .catch(console.error);
        }
        return mainWindow;
    }

    focus() {
        this.window.focus();
    }
}
