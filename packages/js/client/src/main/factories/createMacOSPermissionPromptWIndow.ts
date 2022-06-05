import { BrowserWindow, shell } from 'electron';
import PageType from '../../render/Pages/PageType';

async function createMacOSPermissionPromptWindow(): Promise<BrowserWindow> {
    const permissionWindow = new BrowserWindow({
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
