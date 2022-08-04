export default class Config {
    startAsDirectHost = false;

    directHostPort = '9051';

    startAsSignalHost = false;

    signalServerReliable = 'example.com:69';

    signalServerUnreliable = 'example.com:420';

    staticPassword: string | null = null;

    promptedForPermissionMacOS = false;

    // frontend

    authUrl = 'https://example.com';
}
