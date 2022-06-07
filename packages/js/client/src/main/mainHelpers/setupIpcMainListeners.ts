import {
    BrowserWindow,
    ipcMain,
    IpcMainEvent,
    screen,
    webContents,
} from 'electron';
import { ButtonMask, Display, rust } from '@screenview/node-interop';
import { toJS } from 'mobx';
import GlobalState from '../GlobalState';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import establishSession from './ipcMainHandler/establishSession';
import Config from '../../common/Config';
import { saveConfig } from './configHelper';
import {
    HostHeight,
    HostSelectionHeight,
    HostSelectionWidth,
    HostWidth,
} from '../../common/contants';
import setHostMenubarPosition from './setHostMenubarPosition';

function findClientBundle(state: GlobalState, id: number) {
    return state.clientBundles.find((b) => b.window?.id === id);
}

function findHostWindow(state: GlobalState, id: number) {
    return [state.directHostWindow, state.signalHostWindow].find(
        (b) => b?.id === id
    );
}

function findHostInstanceFromBrowserWindowId(
    state: GlobalState,
    id: number
): rust.HostInstance | null | undefined {
    if (state.directHostWindow?.id === id) {
        return state.directHostInstance;
    }
    if (state.signalHostWindow?.id === id) {
        return state.signalHostInstance;
    }
    return undefined;
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
        const hostWindow = findHostWindow(state, event.sender.id);
        if (!hostWindow) {
            throw new Error('Logical Error: Could not find host window');
        }
        hostWindow.setSize(HostSelectionWidth, HostSelectionHeight, true);
        hostWindow.setMinimumSize(HostSelectionWidth, HostSelectionHeight);
        hostWindow.setResizable(true);

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
            event.reply(MainToRendererIPCEvents.Host_DesktopList, thumbnails);
        });

        function stopHandle(stopEvent?: IpcMainEvent) {
            if (stopEvent && stopEvent.sender.id !== event.sender.id) {
                return;
            }
            if (handle) {
                rust.close_thumbnails(handle);
                handle = undefined;
            }
            if (hostWindow) {
                hostWindow.setMinimumSize(HostWidth, HostHeight);
                hostWindow.setSize(HostWidth, HostHeight, true);
                setHostMenubarPosition(hostWindow);
                hostWindow.setResizable(false);
                hostWindow.webContents.removeListener(
                    'did-start-loading',
                    stopHandle
                );
            }
            ipcMain.removeListener(
                RendererToMainIPCEvents.Host_StopDesktopList,
                stopHandle
            );
        }

        hostWindow.webContents.once('did-start-loading', stopHandle); // in case the user reloads the page (or developer mode)

        ipcMain.on(RendererToMainIPCEvents.Host_StopDesktopList, stopHandle);
    });

    ipcMain.on(
        RendererToMainIPCEvents.Host_UpdateDesktopList,
        async (event, frames: Display[]) => {
            const host = findHostInstanceFromBrowserWindowId(
                state,
                event.sender.id
            );

            if (!host) {
                return;
                throw new Error('Logical Error: Could not find host instance');
            }

            await rust.share_displays(host, frames);
        }
    );

    ipcMain.on(RendererToMainIPCEvents.Host_Disconnect, async (event) => {
        const host = findHostInstanceFromBrowserWindowId(
            state,
            event.sender.id
        );

        if (!host) {
            return;
            throw new Error('Logical Error: Could not find host instance');
        }

        // TODO call rust thing to end session

        const window = findHostWindow(state, event.sender.id);

        if (!window) {
            throw new Error('Logical Error: Could not find host window');
        }

        if (window.id === state.directHostWindow?.id) {
            state.directHostWindow = null;
        }

        if (window.id === state.signalHostWindow?.id) {
            state.signalHostWindow = null;
        }

        window.close();
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

    ipcMain.handle(
        RendererToMainIPCEvents.MacOSPPermission_Accessibility,
        (event, prompt: boolean) => rust.macos_accessibility_permission(prompt)
    );

    ipcMain.handle(RendererToMainIPCEvents.MacOSPPermission_ScreenCapture, () =>
        rust.macos_screen_capture_permission()
    );

    ipcMain.handle(
        RendererToMainIPCEvents.MacOSPPermission_ScreenCapturePrompt,
        () => rust.macos_screen_capture_permission_prompt()
    );
}
