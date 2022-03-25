import { app, BrowserWindow, shell, Tray } from 'electron';
import createMenu from './menu';
import createMainWindow from './mainWindow';
import createToolBox from './createToolBox';

app.on('ready', () => {
    createToolBox();
    // createMainWindow();
    // createMenu(null);

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) createMainWindow();
    });
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});
