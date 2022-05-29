import { BrowserWindow, shell } from 'electron';
import { InstanceConnectionType } from 'node-interop';
import PageType from '../../render/Pages/PageType';
import { HostHeight, HostWidth } from '../contants';

async function createHostWindow(
    type: InstanceConnectionType
): Promise<BrowserWindow> {
    const hostWindow = new BrowserWindow({
        height: HostHeight,
        width: HostWidth,
        resizable: false,
        frame: false,
        alwaysOnTop: true,
        transparent: true,
        roundedCorners: false,
        webPreferences: {
            nodeIntegration: true,
            contextIsolation: false,
        },
    });

    hostWindow.center();
    hostWindow.setPosition(hostWindow.getPosition()[0], 0, true);

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
