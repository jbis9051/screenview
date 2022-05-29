import { ipcMain, webContents } from 'electron';
import { ButtonMask, InstanceConnectionType, rust } from 'node-interop';
import { toJS } from 'mobx';
import GlobalState from '../GlobalState';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import establishSession from './ipcMainHandler/establishSession';
import Config from '../../common/Config';
import { saveConfig } from './configHelper';

function findClientBundle(state: GlobalState, id: number) {
    return state.clientBundles.find((b) => b.window?.id === id);
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

    ipcMain.on(RendererToMainIPCEvents.Host_GetDesktopList, async (event) => {
        let handle: rust.ThumbnailHandle | undefined;

        handle = rust.thumbnails((thumbnails) => {
            const contents = webContents.fromId(event.sender.id); // if the window is closed without canceling the thumbnails, we check if the webcontents id still exists
            if (!contents && handle) {
                // if the webcontents id does not exist, we cancel the thumbnails
                rust.close_thumbnails(handle);
                handle = undefined;
                return;
            }
            // otherwise just send the thumbnails
            event.sender.send(
                MainToRendererIPCEvents.Host_DesktopList,
                thumbnails
            );
        });

        ipcMain.on(
            RendererToMainIPCEvents.Host_StopDesktopList,
            (stopEvent) => {
                if (stopEvent.sender.id === event.sender.id && handle) {
                    rust.close_thumbnails(handle);
                    handle = undefined;
                }
            }
        );
    });

    ipcMain.on(RendererToMainIPCEvents.Main_ConfigRequest, (event) => {
        event.sender.send(
            MainToRendererIPCEvents.Main_ConfigResponse,
            toJS(state.config)
        );
    });

    ipcMain.on(
        RendererToMainIPCEvents.Main_ConfigUpdate,
        async (_, config: Config) => {
            state.config = config;
            await saveConfig(config);
        }
    );
}
