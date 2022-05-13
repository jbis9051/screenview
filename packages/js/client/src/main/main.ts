import { app, BrowserWindow, ipcMain } from 'electron';
import { rust, InstanceConnectionType, InstancePeerType } from 'node-interop';
import GlobalState from './GlobalState';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../common/IPCEvents';
import createTray from './actions/createTray';
import createClientWindow from './factories/createClientWindow';
import connectInstanceToSignal from './interopHelpers/connectInstanceToSignal';
import VTableEmitter, { VTableEvent } from './interopHelpers/VTableEmitter';
import startMainWindow from './mainHelpers/startMainWindow';
import setupReactions from './mainHelpers/setupReactions';

const state = new GlobalState();

setupReactions(state);

// SVSC

ipcMain.on(RendererToMainIPCEvents.EstablishSession, async (_, id: string) => {
    const emitter = new VTableEmitter();
    const window = await createClientWindow();
    const instance = rust.new_instance(
        InstancePeerType.Client,
        InstanceConnectionType.Signal,
        emitter
    );
    state.clientBundles.push({
        instance,
        window,
        emitter,
    });
    emitter.on(VTableEvent.SessionUpdate, () => {
        window.webContents.send(MainToRendererIPCEvents.SessionUpdate);
    });
    await connectInstanceToSignal(state, instance);
    await rust.establish_session(instance, id);
});

app.on('ready', async () => {
    // TODO load config from preferences

    await startMainWindow(state);
    await createTray(state);

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) startMainWindow(state);
    });
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});
