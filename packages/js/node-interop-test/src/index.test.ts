import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
} from '@screenview/node-interop';
import VTableMocker, { VTableEvent } from './VTableMocker';

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
    const vtableHost = new VTableMocker();
    const vtableClient = new VTableMocker();

    const host = rust.new_instance(
        InstancePeerType.Host,
        InstanceConnectionType.Direct,
        vtableHost
    );

    await rust.start_server(host, '0.0.0.0:9051');

    await rust.update_static_password(host, 'password');

    const client = rust.new_instance(
        InstancePeerType.Client,
        InstanceConnectionType.Direct,
        vtableClient
    );

    await rust.connect(client, ConnectionType.Reliable, '127.0.0.1:9051');

    await vtableClient.wait_for_event(
        VTableEvent.WpsskaClientPasswordPrompt,
        500
    );

    await rust.process_password(client, 'password');

    await Promise.all([
        vtableClient.wait_for_event(
            VTableEvent.WpsskaClientAuthenticationSuccessful,
            500
        ),
        vtableHost.wait_for_event(
            VTableEvent.WpsskaClientAuthenticationSuccessful,
            500
        ),
    ]);

    const displays = rust.available_displays();

    expect(displays.length).toBeGreaterThan(0);

    const firstMonitor = displays.find((display) => display.type === 'monitor');

    expect(firstMonitor).toBeDefined();

    await rust.share_displays(host, [firstMonitor!]);

    const [id, data] = (await vtableClient.wait_for_event(
        VTableEvent.RvdFrameData,
        500
    )) as [number, ArrayBuffer];

    expect(id).toBe(0);
    expect(data.byteLength).toBeGreaterThan(0);
});
