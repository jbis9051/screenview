import { ipcRenderer } from 'electron';
import { action } from 'mobx';
import { DisplayShare, VTableEvent } from '@screenview/node-interop';
import { MainToRendererIPCEvents } from '../../../common/IPCEvents';
import UIStore, { ConnectionStatus } from '../../store/Client/UIStore';
import { handleFrame, handleFrameError } from './handleFrame';

function handleRvdFrame(
    displayId: number,
    vp9: ArrayBuffer,
    timestamp: number,
    key: boolean
) {
    console.log('handleRvdFrame', displayId, vp9.byteLength, timestamp, key);
    const decoder = UIStore.decoder.get(displayId);
    if (!decoder) {
        throw new Error(`Decoder not found for displayId: ${displayId}`);
    }
    const chunk = new EncodedVideoChunk({
        timestamp,
        type: Math.random() > 0.5 ? 'key' : 'delta',
        data: new Uint8Array(vp9),
    });
    decoder.decode(chunk);
}

export default function setUpClientListeners() {
    ipcRenderer.on(
        MainToRendererIPCEvents.Client_VTableEvent,
        (_, event, ...args: any[]) => {
            switch (event) {
                case VTableEvent.WpsskaClientAuthenticationFailed:
                    UIStore.connectionStatus = ConnectionStatus.Error;
                    UIStore.error = `An Error Occurred While connecting: ${args[0]}`;
                    break;
                case VTableEvent.WpsskaClientAuthenticationSuccessful:
                    UIStore.connectionStatus = ConnectionStatus.Handshaking;
                    break;
                case VTableEvent.RvdClientHandshakeComplete:
                    UIStore.connectionStatus = ConnectionStatus.Connected;
                    break;
                case VTableEvent.RvdDisplayShare: {
                    console.log('GOT A DISPLAY SHARE');
                    const share = args[0] as DisplayShare;
                    UIStore.displayShares.push(share);
                    const decoder = new VideoDecoder({
                        output: (frame) => handleFrame(share.display_id, frame),
                        error: (error) =>
                            handleFrameError(share.display_id, error),
                    });
                    decoder.configure({ codec: 'vp09.00.41.08' });
                    UIStore.decoder.set(share.display_id, decoder);
                    break;
                }
                case VTableEvent.RvdDisplayUnshare: {
                    const displayId = args[0] as number;
                    UIStore.decoder.delete(displayId);
                    UIStore.displayShares = UIStore.displayShares.filter(
                        (share) => share.display_id !== displayId
                    );
                    break;
                }
                case VTableEvent.RvdClientFrameData: {
                    const [id, width, height, data] = args;
                    const frame = new VideoFrame(data, {
                        format: 'I420',
                        codedWidth: width,
                        codedHeight: height,
                        timestamp: 0,
                    });
                    handleFrame(id, frame);
                    break;
                }
                default:
                    console.log('Unknown VTable Event', event);
                    break;
            }
        }
    );
}
