import { makeAutoObservable } from 'mobx';

class ConfigStore {
    authUrl = 'https://example.com';

    constructor() {
        makeAutoObservable(this);
    }
}
export default new ConfigStore();
