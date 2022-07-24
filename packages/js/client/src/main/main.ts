import { app, BrowserWindow } from 'electron';
import FileBasedConfigurationService from './Services/FileBasedConfigurationService';
import Application from './Application';

const configService = new FileBasedConfigurationService();
let application: Application<FileBasedConfigurationService> | null = null;

app.on('ready', async () => {
    application = await Application.new(configService);
    application.start();
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
