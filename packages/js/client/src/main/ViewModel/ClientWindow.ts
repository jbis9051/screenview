import { shell, ipcMain, IpcMainEvent, BrowserWindow } from 'electron';
import events from 'events';
import PageType from '../../render/Pages/PageType';
import { RendererToMainIPCEvents } from '../../common/IPCEvents';

export default class ClientWindow extends events.EventEmitter {
    window: BrowserWindow;

    private cleanup: Array<() => void> = [];

    constructor() {
        super();
        this.window = ClientWindow.createWindow();
        this.setupListeners();
        this.window.on('closed', () => {
            this.cleanup.forEach((cleanup) => cleanup());
        });
    }

    private static createWindow() {
        const clientWindow = new BrowserWindow({
            height: 550,
            width: 950,
            minHeight: 550,
            minWidth: 900,
            titleBarStyle: 'hidden',
            webPreferences: {
                nodeIntegration: true,
                contextIsolation: false,
            },
        });

        clientWindow.webContents.addListener('new-window', (e, url) => {
            e.preventDefault();
            return shell.openExternal(url);
        });

        if (process.env.NODE_ENV === 'development') {
            clientWindow
                .loadURL(`http://localhost:8080/#${PageType.Client}`)
                .catch(console.error);
        }
        return clientWindow;
    }

    private setupListeners() {
        const forward = [
            RendererToMainIPCEvents.Client_PasswordInput,
            RendererToMainIPCEvents.Client_MouseInput,
            RendererToMainIPCEvents.Client_KeyboardInput,
        ];
        forward.forEach((event) => {
            const cb = (e: IpcMainEvent, ...args: any[]) => {
                if (e.sender.id !== this.window.id) {
                    return;
                }
                this.emit(event, ...args);
            };
            ipcMain.on(event, cb);
            this.cleanup.push(() => ipcMain.removeListener(event, cb));
        });
    }
}
