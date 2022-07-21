import { app, BrowserWindow } from 'electron';
import { rust } from '@screenview/node-interop';
import { macos_accessibility_permission } from '@screenview/node-interop/index.node';
import GlobalState from './GlobalState';
import startMainWindow from './helpers/startMainWindow';
import setupReactions from './helpers/setupReactions';
import setupIpcMainListeners from './helpers/setupIpcMainListeners';
import { loadConfig, saveConfig } from './helpers/configHelper';
import Config from '../common/Config';
import createHostWindow from './factories/createHostWindow';
import createMacOSPermissionPromptWindow from './factories/createMacOSPermissionPromptWIndow';
import createTray from './actions/createTray';

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
    // On macOS 10.15+ we must request permission to access the screen and accessibility API. Both are used for Hosting. Screen access changes requires the app to be restarted.
    if (
        process.platform === 'darwin' &&
        !state.config.promptedForPermissionMacOS
    ) {
        const accessibilityPermission =
            rust.macos_accessibility_permission(false);
        const screenCapturePermission = rust.macos_screen_capture_permission();
        if (!accessibilityPermission || !screenCapturePermission) {
            state.config.promptedForPermissionMacOS = true;
            await saveConfig(state.config);
            const permissionWindow = await createMacOSPermissionPromptWindow();
            permissionWindow.on('closed', () => {
                startMainWindow(state);
            });
            return;
        }
    }

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
