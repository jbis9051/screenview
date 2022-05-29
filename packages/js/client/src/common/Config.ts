import { app } from 'electron';
import path from 'path';
import fs from 'fs/promises';

export default class Config {
    startAsDirectHost = false; // TODO change to true once this works

    startAsSignalHost = false; // TODO change to true once this works

    signalServerReliable = 'example.com:69';

    signalServerUnreliable = 'example.com:420';

    staticPassword: string | null = null;

    // frontend

    authUrl = 'https://example.com';
}
