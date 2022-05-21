import { makeAutoObservable } from 'mobx';
import { InstanceConnectionType } from 'node-interop';

class BackendState {
    type: InstanceConnectionType | null = null;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new BackendState();
