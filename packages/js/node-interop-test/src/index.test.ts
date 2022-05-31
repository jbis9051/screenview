import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
    VTableEvent,
} from '@screenview/node-interop';

function waitForEvent(
    emitter: VTableEmitter,
    event: VTableEvent,
    timeout = -1
): Promise<any[]> {
    return new Promise((resolve) => {
        let expireTimeout: NodeJS.Timeout | null = null;
        const handleEvent = (e: Event, ...arg: any[]) => {
            if (expireTimeout) {
                clearTimeout(expireTimeout);
            }
            if (e !== event) {
                throw new Error(`Expected event ${event}, got ${e}`);
            }
            resolve(arg);
        };
        emitter.once('event', handleEvent);
        if (timeout > 0) {
            expireTimeout = setTimeout(() => {
                emitter.removeListener('event', handleEvent);
                throw new Error(`Timeout waiting for event ${event}`);
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

test('direct connection', async () => {
    const vtableHost = new VTableEmitter();
    const vtableClient = new VTableEmitter();

    const host = rust.new_instance(
        InstancePeerType.Host,
        InstanceConnectionType.Direct,
        vtableHost
    );

    await rust.start_server(host, '127.0.0.1:9051');

    await rust.update_static_password(host, 'password');

    const client = rust.new_instance(
        InstancePeerType.Client,
        InstanceConnectionType.Direct,
        vtableClient
    );

    await rust.connect(client, ConnectionType.Reliable, '127.0.0.1:9051');

    await waitForEvent(
        vtableClient,
        VTableEvent.WpsskaClientPasswordPrompt,
        500
    );

    await rust.process_password(client, 'password');

    await Promise.all([
        waitForEvent(
            vtableClient,
            VTableEvent.WpsskaClientAuthenticationSuccessful,
            500
        ),
        waitForEvent(
            vtableHost,
            VTableEvent.WpsskaClientAuthenticationSuccessful,
            500
        ),
    ]);

    const displays = rust.available_displays();

    expect(displays.length).toBeGreaterThan(0);

    const firstMonitor = displays.find((display) => display.type === 'monitor');

    expect(firstMonitor).toBeDefined();

    await rust.share_displays(host, [firstMonitor!]);

    const [id, data] = (await waitForEvent(
        vtableClient,
        VTableEvent.RvdFrameData,
        500
    )) as [number, ArrayBuffer];

    expect(id).toBe(0);
    expect(data.byteLength).toBeGreaterThan(0);
});
