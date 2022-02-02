import { makeAutoObservable } from 'mobx';

class UserStore {
    user = null;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UserStore();
