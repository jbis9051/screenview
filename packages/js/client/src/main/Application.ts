import { InstanceConnectionType } from '@screenview/node-interop';
import HostManager from './Controllers/HostManager';
import ConfigurationService from './Services/ConfigurationService';
import Config from '../common/Config';
import ClientManager from './Controllers/ClientManager';
import MainManager from './Controllers/MainManager';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../common/IPCEvents';
import IpcListenerService from './Services/IpcListenerService';

export default class Application<T extends ConfigurationService> {
    configurationService: T;

    config: Config;

    mainManager = new MainManager();

    hostSignalManger: HostManager<InstanceConnectionType.Signal> | null = null;

    hostDirectManger: HostManager<InstanceConnectionType.Direct> | null = null;

    clientMangers: ClientManager<
        InstanceConnectionType.Direct | InstanceConnectionType.Signal
    >[] = [];

    listenerService = new IpcListenerService();

    private constructor(configService: T, config: Config) {
        this.configurationService = configService;
        this.config = config;
        this.setupListeners();
    }

    static async new<T extends ConfigurationService>(
        configService: T
    ): Promise<Application<T>> {
        const config = await configService.load();
        return new Application<T>(configService, config);
    }

    async init() {
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
        this.mainManager.focus();
        await this.startHosts();
    }

    focus() {
        this.mainManager.focus();
    }

    private setupListeners() {
        this.listenerService.listen(
            RendererToMainIPCEvents.Main_ConfigRequest,
            (event) => {
                event.reply(
                    MainToRendererIPCEvents.Main_ConfigResponse,
                    this.config
                );
            }
        );

        this.listenerService.listen(
            RendererToMainIPCEvents.Main_EstablishSession,
            async (_, id) => {
                const manager = await ClientManager.new(
                    id,
                    this.listenerService,
                    () => {
                        manager.onDestroy();
                        this.clientMangers = this.clientMangers.filter(
                            (m) => m !== manager
                        );
                    }
                );
                this.clientMangers.push(manager);
            }
        );
    }

    private updateConfig(config: Config) {
        this.config = config;
        this.startHosts();
    }

    private async startHosts() {
        if (this.config.startAsSignalHost) {
            if (!this.hostSignalManger) {
                this.hostSignalManger = new HostManager(
                    InstanceConnectionType.Signal,
                    this.listenerService
                );
            }
        } else if (this.hostSignalManger) {
            this.hostSignalManger.onDestroy();
            this.hostSignalManger = null;
        }
        if (this.config.startAsDirectHost) {
            if (!this.hostDirectManger) {
                this.hostDirectManger = new HostManager(
                    InstanceConnectionType.Direct,
                    this.listenerService,
                    this.config.directHostPort
                );
                await this.hostDirectManger.init();
            }
        } else if (this.hostDirectManger) {
            this.hostDirectManger.onDestroy();
            this.hostDirectManger = null;
        }
    }
}
