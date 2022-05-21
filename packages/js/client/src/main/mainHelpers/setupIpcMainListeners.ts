import { ipcMain } from 'electron';
import { ButtonMask, InstanceConnectionType, rust } from 'node-interop';
import GlobalState from '../GlobalState';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import establishSession from './ipcMainHandler/establishSession';

function findClientBundle(state: GlobalState, id: number) {
    return state.clientBundles.find((b) => b.window?.webContents.id === id);
}

export default function setupIpcMainListeners(state: GlobalState) {
    ipcMain.on(RendererToMainIPCEvents.Main_EstablishSession, (_, id: string) =>
        establishSession(state, id)
    );

    ipcMain.on(
        RendererToMainIPCEvents.Client_PasswordInput,
        async (event, password: string) => {
            const bundle = findClientBundle(state, event.sender.id);
            if (!bundle) {
                console.error(
                    'Could not find bundle for password input. This maybe a bug. Not too sure.'
                );
                return;
            }
            await rust.process_password(bundle.instance, password);
        }
    );

    ipcMain.on(
        RendererToMainIPCEvents.Client_MouseInput,
        async (
            event,
            x: number,
            y: number,
            buttonMask: ButtonMask,
            buttonMaskState: ButtonMask
        ) => {
            const bundle = findClientBundle(state, event.sender.id);
            if (!bundle) {
                console.error(
                    'Could not find bundle for mouse input. This maybe a bug. Not too sure.'
                );
                return;
            }
            await rust.mouse_input(
                bundle.instance,
                x,
                y,
                buttonMask,
                buttonMaskState
            );
        }
    );

    ipcMain.on(
        RendererToMainIPCEvents.Client_KeyboardInput,
        async (event, keyCode: number, down: boolean) => {
            const bundle = findClientBundle(state, event.sender.id);
            if (!bundle) {
                console.error(
                    'Could not find bundle for keyboard input. This maybe a bug. Not too sure.'
                );
                return;
            }
            await rust.keyboard_input(bundle.instance, keyCode, down);
        }
    );

    ipcMain.on(
        RendererToMainIPCEvents.Host_GetDesktopList,
        async (_, whichHost: InstanceConnectionType) => {
            let host: rust.HostInstance | null = null;
            if (whichHost === InstanceConnectionType.Direct) {
                host = state.directHostInstance;
            }
            if (whichHost === InstanceConnectionType.Signal) {
                host = state.signalHostInstance;
            }
            if (!host) {
                state.mainWindow?.webContents.send(
                    MainToRendererIPCEvents.DesktopList,
                    null
                );
                throw new Error(
                    "Host type doesn't exist. So I can't get desktop list."
                );
            }
            const thumbnails = await rust.thumbnails(host);
            state.mainWindow?.webContents.send(
                MainToRendererIPCEvents.DesktopList,
                thumbnails
            );
        }
    );
}
