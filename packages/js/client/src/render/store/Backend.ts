import { makeAutoObservable } from 'mobx';

class BackendState {
    status = false;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new BackendState();
