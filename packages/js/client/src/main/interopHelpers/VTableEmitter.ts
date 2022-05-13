import { rust } from 'node-interop';
import { EventEmitter } from 'events';

export enum VTableEvent {
    SessionIdUpdate = 'session_id_update',
    SessionUpdate = 'session_update',
}

export default class VTableEmitter extends EventEmitter implements rust.VTable {
    session_id_update(sessionId: string): void {
        if (this.listeners(VTableEvent.SessionIdUpdate).length === 0) {
            throw new Error(
                `No listeners for ${VTableEvent.SessionIdUpdate}, this probably shouldn't have been emitted`
            );
        }
        this.emit(VTableEvent.SessionIdUpdate, sessionId);
    }

    session_update(): void {
        if (this.listeners(VTableEvent.SessionUpdate).length === 0) {
            throw new Error(
                `No listeners for ${VTableEvent.SessionUpdate}, this probably shouldn't have been emitted`
            );
        }
        this.emit(VTableEvent.SessionUpdate);
    }
}
