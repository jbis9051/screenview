import { app, BrowserWindow, shell } from 'electron';

function createWindow() {
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
        mainWindow.loadURL('http://localhost:8080');
    } else {
        // mainWindow.loadFile(path.join(__dirname, '../index.html'));
    }
}

app.on('ready', () => {
    createWindow();
    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) createWindow();
    });
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});
