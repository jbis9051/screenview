import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
    VTableEvent,
} from '@screenview/node-interop';
import * as console from 'console';

function waitForEvent(
    emitter: VTableEmitter,
    event: VTableEvent,
    timeout = -1
): Promise<any[]> {
    return new Promise((resolve, reject) => {
        let expireTimeout: NodeJS.Timeout | null = null;
        const handleEvent = (e: Event, ...arg: any[]) => {
            if (expireTimeout) {
                clearTimeout(expireTimeout);
            }
            if (e !== event) {
                reject(new Error(`Expected event ${event}, got ${e}`));
                return;
            }
            resolve(arg);
        };
        emitter.once('event', handleEvent);
        if (timeout > 0) {
            expireTimeout = setTimeout(() => {
                emitter.removeListener('event', handleEvent);
                reject(new Error(`Timeout waiting for event ${event}`));
            }, timeout);
        }
    });
}

test('thumbnails', (done) => {
    let thumbnailHandle: rust.ThumbnailHandle | null = null;
    thumbnailHandle = rust.thumbnails((thumbs) => {
        expect(thumbs.length).toBeGreaterThan(0);

        if (thumbnailHandle) {
            rust.close_thumbnails(thumbnailHandle);
        }

        done();
    });
});

jest.setTimeout(100000);
jest.useRealTimers();

test('direct connection', async () => {
    const vtableHost = new VTableEmitter();
    const vtableClient = new VTableEmitter();

    const host = rust.new_instance(
        InstancePeerType.Host,
        InstanceConnectionType.Direct,
        vtableHost
    );

    await rust.start_server(host, '127.0.0.1:9051', '127.0.0.1:9051');
    await rust.update_static_password(host, 'password');

    const client = rust.new_instance(
        InstancePeerType.Client,
        InstanceConnectionType.Direct,
        vtableClient
    );

    await rust.connect(client, ConnectionType.Reliable, '127.0.0.1:9051');
    await rust.connect(client, ConnectionType.Unreliable, '127.0.0.1:9051');

    await waitForEvent(
        vtableClient,
        VTableEvent.WpsskaClientPasswordPrompt,
        2000
    );

    await Promise.all([
        rust.process_password(client, 'password'),
        waitForEvent(
            vtableClient,
            VTableEvent.WpsskaClientAuthenticationSuccessful,
            5000
        ),
        waitForEvent(
            vtableHost,
            VTableEvent.WpsskaHostAuthenticationSuccessful,
            5000
        ),
    ]);

    await Promise.all([
        waitForEvent(
            vtableClient,
            VTableEvent.RvdClientHandshakeComplete,
            5000
        ),
        waitForEvent(vtableHost, VTableEvent.RvdHostHandshakeComplete, 5000),
    ]);

    const displays = rust.available_displays();

    expect(displays.length).toBeGreaterThan(0);

    const firstMonitor = displays.find((display) => display.type === 'monitor');

    expect(firstMonitor).toBeDefined();

    await rust.share_displays(host, [firstMonitor!], false);

    await waitForEvent(vtableClient, VTableEvent.RvdDisplayShare, 5000);

    const [id, width, height, data] = (await waitForEvent(
        vtableClient,
        VTableEvent.RvdClientFrameData,
        5000
    )) as [number, number, number, ArrayBuffer];

    expect(id).toBe(0);
    expect(width).toBeGreaterThan(0);
    expect(height).toBeGreaterThan(0);
    expect(data.byteLength).toBeGreaterThan(0);
    await rust.share_displays(host, [], false);
    rust.close_instance(client);
    rust.close_instance(host);
});
