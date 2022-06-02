import { BrowserWindow, ipcMain } from 'electron';
import { RendererToMainIPCEvents } from '../../common/IPCEvents';

export default function waitForReadySignal(
    window: BrowserWindow,
    timeout: number | null = 1000
): Promise<void> {
    return new Promise((resolve, reject) => {
        let timeoutId: NodeJS.Timeout | null = null;

        function handle() {
            if (timeoutId) {
                clearTimeout(timeoutId);
            }
            resolve();
        }

        if (timeout) {
            timeoutId = setTimeout(() => {
                ipcMain.removeListener(
                    RendererToMainIPCEvents.RendererReady,
                    handle
                );
                reject(
                    new Error(
                        "Render Ready wasn't received in time. Either you added this listener too late or the renderer didn't load."
                    )
                );
            }, timeout);
        }

        ipcMain.once(RendererToMainIPCEvents.RendererReady, handle);
    });
}
