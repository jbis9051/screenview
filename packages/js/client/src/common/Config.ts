export default class Config {
    startAsDirectHost = false; // TODO change to true once this works

    startAsSignalHost = false; // TODO change to true once this works

    signalServerReliable = 'example.com:69';

    signalServerUnreliable = 'example.com:420';

    staticPassword: string | null = null;

    promptedForPermissionMacOS = false;

    // frontend

    authUrl = 'https://example.com';
}
