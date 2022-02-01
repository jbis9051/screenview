import { app, BrowserWindow } from 'electron';

function createWindow() {
    const mainWindow = new BrowserWindow({
        height: 550,
        width: 950,
        titleBarStyle: 'hidden',
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
