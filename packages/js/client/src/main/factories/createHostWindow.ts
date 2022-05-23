import { BrowserWindow, shell } from 'electron';
import { InstanceConnectionType } from 'node-interop';
import PageType from '../../render/Pages/PageType';

async function createHostWindow(
    type: InstanceConnectionType
): Promise<BrowserWindow> {
    const hostWindow = new BrowserWindow({
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

    hostWindow.webContents.addListener('new-window', (e, url) => {
        e.preventDefault();
        return shell.openExternal(url);
    });

    if (process.env.NODE_ENV === 'development') {
        hostWindow
            .loadURL(
                `http://localhost:8080/#${
                    type === InstanceConnectionType.Signal
                        ? PageType.SignalHost
                        : PageType.DirectHost
                }`
            )
            .catch(console.error);
    }
    return hostWindow;
    // hostWindow.loadFile(path.join(__dirname, '../index.html'));
}
export default createHostWindow;
