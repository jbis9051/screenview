import { ipcMain } from 'electron';
import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
} from 'node-interop';
import { action } from 'mobx';
import GlobalState from '../GlobalState';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import VTableEmitter, { VTableEvent } from '../interopHelpers/VTableEmitter';
import createClientWindow from '../factories/createClientWindow';
import establishSession from './ipcMainHandler/establishSession';

export default function setupIpcMainListeners(state: GlobalState) {
    ipcMain.on(RendererToMainIPCEvents.EstablishSession, (_, id: string) =>
        establishSession(state, id)
    );
}
