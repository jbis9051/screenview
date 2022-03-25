import { app, BrowserWindow, shell, Tray } from 'electron';
import createMenu from './menu';
import createMainWindow from './mainWindow';

app.on('ready', () => {
    createMainWindow();
    createMenu(null);

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) createMainWindow();
    });
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});
