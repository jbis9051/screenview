import { app } from 'electron';
import path from 'path';
import fs from 'fs/promises';
import Config from '../../common/Config';

const userData = app.getPath('userData');
const dataPath = path.join(userData, 'screenview');
export const preferencesPath = path.join(dataPath, 'preferences.json');

export async function saveConfig(config: Config) {
    await fs.mkdir(dataPath, { recursive: true });
    await fs.writeFile(preferencesPath, JSON.stringify(config));
    console.log('Saved preferences to', preferencesPath);
}

export function loadConfig(): Promise<Config> {
    return fs.readFile(preferencesPath, 'utf8').then((data) => {
        const saved = JSON.parse(data);
        const tmp = new Config();
        return {
            ...tmp,
            ...saved,
        };
    });
}
