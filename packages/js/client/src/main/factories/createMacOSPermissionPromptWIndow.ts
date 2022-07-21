import { BrowserWindow, shell } from 'electron';
import PageType from '../../render/Pages/PageType';
import startMainWindow from '../helpers/startMainWindow';

async function createMacOSPermissionPromptWindow(): Promise<BrowserWindow> {
    const permissionWindow = new BrowserWindow({
        height: 700,
        width: 600,
        resizable: false,
        titleBarStyle: 'hidden',
        webPreferences: {
            nodeIntegration: true,
            contextIsolation: false,
        },
    });

    permissionWindow.webContents.addListener('new-window', (e, url) => {
        e.preventDefault();
        return shell.openExternal(url);
    });

    if (process.env.NODE_ENV === 'development') {
        permissionWindow
            .loadURL(`http://localhost:8080/#${PageType.MacOSPermission}`)
            .catch(console.error);
    }
    return permissionWindow;
    // permissionWindow.loadFile(path.join(__dirname, '../index.html'));
}
export default createMacOSPermissionPromptWindow;
