import { app, BrowserWindow, shell } from 'electron';
import PageType from '../render/Pages/PageType';

// eslint-disable-next-line consistent-return
function createMainWindow() {
    const mainWindow = new BrowserWindow({
        height: 550,
        width: 950,
        minHeight: 550,
        minWidth: 900,
        titleBarStyle: 'hidden',
    });

    mainWindow.webContents.addListener('new-window', (e, url) => {
        e.preventDefault();
        return shell.openExternal(url);
    });

    if (process.env.NODE_ENV === 'development') {
        return mainWindow.loadURL(`http://localhost:8080/#${PageType.HOME}`);
    }
    // mainWindow.loadFile(path.join(__dirname, '../index.html'));
}

app.on('ready', () => {
    createMainWindow();
    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) createMainWindow();
    });
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});
