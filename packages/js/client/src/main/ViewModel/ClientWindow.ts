import { shell, BrowserWindow } from 'electron';
import PageType from '../../render/Pages/PageType';

export default class ClientWindow {
    window: BrowserWindow;

    constructor(onClose: () => void) {
        this.window = ClientWindow.createWindow();
        this.window.on('closed', onClose);
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
}
