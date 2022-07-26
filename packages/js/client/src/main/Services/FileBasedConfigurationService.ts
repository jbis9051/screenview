import path from 'path';
import { app } from 'electron';
import fs from 'fs/promises';
import ConfigurationService from './ConfigurationService';
import Config from '../../common/Config';

const userData = app.getPath('userData');
const dataPath = path.join(userData, 'screenview');
const preferencesPath = path.join(dataPath, 'preferences.json');

export default class FileBasedConfigurationService
    implements ConfigurationService
{
    // eslint-disable-next-line class-methods-use-this
    async load(): Promise<Config> {
        return fs
            .readFile(preferencesPath, 'utf8')
            .then((data) => {
                const saved = JSON.parse(data);
                const tmp = new Config();
                return {
                    ...tmp,
                    ...saved,
                };
            })
            .catch(() => new Config());
    }

    // eslint-disable-next-line class-methods-use-this
    async save(config: Config): Promise<void> {
        await fs.mkdir(dataPath, { recursive: true });
        await fs.writeFile(preferencesPath, JSON.stringify(config));
    }
}
