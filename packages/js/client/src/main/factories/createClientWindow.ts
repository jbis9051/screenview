import { BrowserWindow, shell } from 'electron';
import PageType from '../../render/Pages/PageType';

async function createClientWindow(): Promise<BrowserWindow> {
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
    // clientWindow.loadFile(path.join(__dirname, '../index.html'));
}
export default createClientWindow;
