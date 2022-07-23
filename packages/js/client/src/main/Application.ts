import { InstanceConnectionType } from '@screenview/node-interop';
import HostManager from './Controllers/HostManager';
import ConfigurationService from './Services/ConfigurationService';
import Config from '../common/Config';
import ClientManager from './Controllers/ClientManager';
import MainManager from './Controllers/MainManager';
import { RendererToMainIPCEvents } from '../common/IPCEvents';

export default class Application<T extends ConfigurationService> {
    configurationService: T;

    config: Config;

    mainManager = new MainManager();

    hostSignalManger: HostManager<InstanceConnectionType.Signal> | null = null;

    hostDirectManger: HostManager<InstanceConnectionType.Direct> | null = null;

    clientMangers: ClientManager<
        InstanceConnectionType.Direct | InstanceConnectionType.Signal
    >[] = [];

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

    start() {
        this.mainManager.focus();
        if (this.config.startAsSignalHost) {
            this.hostSignalManger = new HostManager(
                InstanceConnectionType.Signal
            );
        }
        if (this.config.startAsDirectHost) {
            this.hostDirectManger = new HostManager(
                InstanceConnectionType.Direct
            );
        }
    }

    focus() {
        this.mainManager.focus();
    }

    private setupListeners() {
        this.mainManager.on(
            RendererToMainIPCEvents.Main_EstablishSession,
            (id) => {
                const client = ClientManager.new(id);
                this.clientMangers.push(client);
            }
        );
        this.mainManager.on(
            RendererToMainIPCEvents.Main_ConfigRequest,
            (cb) => {
                cb(this.config);
            }
        );
    }
}
