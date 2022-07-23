import { ipcMain, shell, BrowserWindow } from 'electron';
import { rust } from '@screenview/node-interop';
import PageType from '../../render/Pages/PageType';
import { RendererToMainIPCEvents } from '../../common/IPCEvents';

export default class MacOSPermissionWindow {
    window: BrowserWindow;

    constructor() {
        this.window = MacOSPermissionWindow.createWindow();
        MacOSPermissionWindow.registerListeners();
    }

    private static registerListeners() {
        ipcMain.handle(
            RendererToMainIPCEvents.MacOSPPermission_Accessibility,
            (event, prompt: boolean) =>
                rust.macos_accessibility_permission(prompt)
        );

        ipcMain.handle(
            RendererToMainIPCEvents.MacOSPPermission_ScreenCapture,
            () => rust.macos_screen_capture_permission()
        );

        ipcMain.handle(
            RendererToMainIPCEvents.MacOSPPermission_ScreenCapturePrompt,
            () => rust.macos_screen_capture_permission_prompt()
        );
    }

    private static createWindow() {
        const permissionWindow = new BrowserWindow({
            height: 700,
            width: 600,
            resizable: false,
            titleBarStyle: 'hidden',
            webPreferences: {
                nodeIntegration: true,
                contextIsolation: false,
            },
        });

        permissionWindow.webContents.addListener('new-window', (e, url) => {
            e.preventDefault();
            return shell.openExternal(url);
        });

        if (process.env.NODE_ENV === 'development') {
            permissionWindow
                .loadURL(`http://localhost:8080/#${PageType.MacOSPermission}`)
                .catch(console.error);
        }
        return permissionWindow;
    }
}
