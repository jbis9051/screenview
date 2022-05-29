import { ipcRenderer } from 'electron';
import { reaction, runInAction, toJS } from 'mobx';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import Config from '../../common/Config';
import ConfigStore from '../store/Main/ConfigStore';

export default async function setupPreferences() {
    ipcRenderer.send(RendererToMainIPCEvents.Main_ConfigRequest);

    await new Promise<void>((resolve) => {
        function handle(_: any, config: Config) {
            ipcRenderer.removeListener(
                MainToRendererIPCEvents.Main_ConfigResponse,
                handle
            );
            runInAction(() => {
                ConfigStore.backend = config;
            });
            resolve();
        }
        ipcRenderer.on(MainToRendererIPCEvents.Main_ConfigResponse, handle);
    });

    reaction(
        () => ConfigStore.backend,
        () => {
            ipcRenderer.send(
                RendererToMainIPCEvents.Main_ConfigUpdate,
                toJS(ConfigStore.backend)
            );
        }
    );
}
