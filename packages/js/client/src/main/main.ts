import { app, BrowserWindow } from 'electron';
import { InstanceConnectionType } from 'node-interop';
import GlobalState from './GlobalState';
import startMainWindow from './mainHelpers/startMainWindow';
import setupReactions from './mainHelpers/setupReactions';
import setupIpcMainListeners from './mainHelpers/setupIpcMainListeners';
import { loadConfig, saveConfig } from './mainHelpers/configHelper';
import Config from '../common/Config';
import createHostWindow from './factories/createHostWindow';

const state = new GlobalState();

setupReactions(state);
setupIpcMainListeners(state);

const storedPreferences = loadConfig().catch(async () => {
    const tmp = new Config();
    await saveConfig(tmp);
    return tmp;
});

app.on('ready', async () => {
    state.config = await storedPreferences;
    await startMainWindow(state);
    // await createTray(state);
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
