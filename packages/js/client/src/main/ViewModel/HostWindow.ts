import events from 'events';
import {
    ipcMain,
    IpcMainEvent,
    screen,
    shell,
    webContents,
    BrowserWindow,
} from 'electron';
import {
    Display,
    InstanceConnectionType,
    InstancePeerType,
    rust,
} from '@screenview/node-interop';
import {
    HostHeight,
    HostSelectionHeight,
    HostSelectionWidth,
    HostWidth,
} from '../../common/contants';
import PageType from '../../render/Pages/PageType';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';

export default class HostWindow extends events.EventEmitter {
    window: BrowserWindow;

    type: PageType;

    constructor(type: InstanceConnectionType) {
        super();
        this.type =
            type === InstanceConnectionType.Signal
                ? PageType.SignalHost
                : PageType.DirectHost;
        this.window = this.createWindow();
        this.window.on('close', () => {
            this.emit('close');
        });
        this.setUpListeners();
    }

    async handleGetDesktopList(event: IpcMainEvent) {
        if (this.window.id !== event.sender.id) {
            return;
        }
        this.window.setSize(HostSelectionWidth, HostSelectionHeight, true);
        this.window.setMinimumSize(HostSelectionWidth, HostSelectionHeight);
        this.window.setResizable(true);

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

        const stopHandle = (stopEvent?: IpcMainEvent) => {
            if (stopEvent && stopEvent.sender.id !== event.sender.id) {
                return;
            }
            if (handle) {
                rust.close_thumbnails(handle);
                handle = undefined;
                this.window.setMinimumSize(HostWidth, HostHeight);
                this.window.setSize(HostWidth, HostHeight, true);
                this.setHostMenubarPosition();
                this.window.setResizable(false);
                this.window.webContents.removeListener(
                    'did-start-loading',
                    stopHandle
                );
            }
            ipcMain.removeListener(
                RendererToMainIPCEvents.Host_StopDesktopList,
                stopHandle
            );
        };

        this.window.webContents.once('did-start-loading', stopHandle); // in case the user reloads the page (or developer mode)

        ipcMain.on(RendererToMainIPCEvents.Host_StopDesktopList, stopHandle);
    }

    async handleUpdateDesktopList(
        event: IpcMainEvent,
        displays: Display[],
        controllable: boolean
    ) {
        if (this.window.id !== event.sender.id) {
            return;
        }
        this.emit(
            RendererToMainIPCEvents.Host_UpdateDesktopList,
            displays,
            controllable
        );
    }

    handleDisconnectButton(event: IpcMainEvent) {
        if (this.window.id !== event.sender.id) {
            return;
        }
        this.emit(RendererToMainIPCEvents.Host_DisconnectButton);
    }

    setUpListeners() {
        ipcMain.on(
            RendererToMainIPCEvents.Host_GetDesktopList,
            this.handleGetDesktopList.bind(this)
        );

        ipcMain.on(
            RendererToMainIPCEvents.Host_UpdateDesktopList,
            this.handleUpdateDesktopList.bind(this)
        );

        ipcMain.on(
            RendererToMainIPCEvents.Host_DisconnectButton,
            this.handleDisconnectButton.bind(this)
        );
    }

    private createWindow(): BrowserWindow {
        const hostWindow = new BrowserWindow({
            height: HostHeight,
            width: HostWidth,
            resizable: false,
            frame: false,
            alwaysOnTop: true,
            transparent: true,
            roundedCorners: false,
            hasShadow: false,
            webPreferences: {
                nodeIntegration: true,
                contextIsolation: false,
            },
        });

        hostWindow.webContents.addListener('new-window', (e, url) => {
            e.preventDefault();
            return shell.openExternal(url);
        });

        if (process.env.NODE_ENV === 'development') {
            hostWindow
                .loadURL(`http://localhost:8080/#${this.type}`)
                .catch(console.error);
        }
        return hostWindow;
    }

    private getScreenFromBrowserWindow() {
        const winBounds = this.window.getBounds();
        return screen.getDisplayNearestPoint({
            x: winBounds.x,
            y: winBounds.y,
        });
    }

    private setHostMenubarPosition() {
        const windowScreen = this.getScreenFromBrowserWindow();
        const screenWidth = windowScreen.size.width;
        this.window.setPosition((screenWidth - HostWidth) / 2, 0, true);
    }
}
