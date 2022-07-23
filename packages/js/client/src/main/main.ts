import { app, BrowserWindow } from 'electron';
import FileBasedConfigurationService from './Services/FileBasedConfigurationService';
import Application from './Application';

const configService = new FileBasedConfigurationService();
let application: Application<FileBasedConfigurationService> | null = null;

app.on('ready', async () => {
    application = await Application.new(configService);
    application.start();

    /* // On macOS 10.15+ we must request permission to access the screen and accessibility API. Both are used for Hosting. Screen access changes requires the app to be restarted.
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
            }
        } */
});

app.on('activate', async () => {
    if (BrowserWindow.getAllWindows().length === 0) {
        application?.focus();
    }
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});
