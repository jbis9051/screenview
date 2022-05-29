import { makeAutoObservable } from 'mobx';
import Config from '../../../common/Config';

class ConfigStore {
    // Adds stuff to the Config class not here. Main is source of truth.
    backend: Config = new Config();

    constructor() {
        makeAutoObservable(this);
    }

    equals(other: Partial<ConfigStore>): boolean {
        // TODO type this better
        return Object.keys(this.backend).every((key) => {
            const _key = key as keyof Config;
            return this.backend[_key] === other.backend![_key];
        });
    }
}
export default new ConfigStore();
