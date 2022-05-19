import { app, BrowserWindow } from 'electron';
import GlobalState from './GlobalState';
import createTray from './actions/createTray';
import startMainWindow from './mainHelpers/startMainWindow';
import setupReactions from './mainHelpers/setupReactions';
import setupIpcMainListeners from './mainHelpers/setupIpcMainListeners';

const state = new GlobalState();

setupReactions(state);
setupIpcMainListeners(state);

app.on('ready', async () => {
    // TODO load config from preferences

    await startMainWindow(state);
    await createTray(state);
});

app.on('activate', async () => {
    if (BrowserWindow.getAllWindows().length === 0) {
        await startMainWindow(state);
    }
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});
