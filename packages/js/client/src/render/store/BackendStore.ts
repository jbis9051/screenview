import { makeAutoObservable } from 'mobx';

class BackendState {
    status = false;

    id: string | null = null;

    password: string | null = null;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new BackendState();
