import { BrowserWindow, shell } from 'electron';
import PageType from '../render/Pages/PageType';

let mainWindow: BrowserWindow | undefined;

async function createMainWindow() {
    if (mainWindow) {
        mainWindow.show();
        return mainWindow;
    }

    mainWindow = new BrowserWindow({
        height: 550,
        width: 950,
        minHeight: 550,
        minWidth: 900,
        titleBarStyle: 'hidden',
    });

    mainWindow.on('close', () => {
        mainWindow = undefined;
    });

    mainWindow.webContents.addListener('new-window', (e, url) => {
        e.preventDefault();
        return shell.openExternal(url);
    });

    if (process.env.NODE_ENV === 'development') {
        await mainWindow.loadURL(`http://localhost:8080/#${PageType.HOME}`);
    }
    return mainWindow;
    // mainWindow.loadFile(path.join(__dirname, '../index.html'));
}
export default createMainWindow;
