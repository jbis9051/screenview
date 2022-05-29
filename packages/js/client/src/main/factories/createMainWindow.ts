import { BrowserWindow, shell } from 'electron';
import PageType from '../../render/Pages/PageType';

async function createMainWindow(): Promise<[Promise<void>, BrowserWindow]> {
    const mainWindow = new BrowserWindow({
        height: 550,
        width: 950,
        minHeight: 550,
        minWidth: 900,
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
        return [
            mainWindow
                .loadURL(`http://localhost:8080/#${PageType.Main}`)
                .catch(console.error),
            mainWindow,
        ];
    }
    return [Promise.resolve(), mainWindow];
    // mainWindow.loadFile(path.join(__dirname, '../index.html'));
}
export default createMainWindow;
