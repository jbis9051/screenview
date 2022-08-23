// handle output from VideoDecoder

import UIStore from '../../store/Client/UIStore';

export function handleFrame(displayId: number, frame: VideoFrame) {
    console.log(frame);
    let canvas = UIStore.canvases.get(displayId);
    if (!canvas) {
        canvas = document.createElement('canvas');
        canvas.style.width = '100%';
        canvas.style.height = '100%';
        canvas.style.position = 'fixed';
        canvas.style.top = '0';
        canvas.style.left = '0';
        document.body.appendChild(canvas);
        UIStore.canvases.set(displayId, canvas);
    }
    canvas.getContext('2d')!.drawImage(frame, 0, 0);
}

export function handleFrameError(displayId: number, error: DOMException) {
    console.log('handleFrameError', displayId, error);
}
