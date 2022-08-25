// handle output from VideoDecoder

import UIStore from '../../store/Client/UIStore';

export function handleFrame(displayId: number, frame: VideoFrame) {
    console.log(frame);
    let canvas = UIStore.canvases.get(displayId);
    if (!canvas) {
        canvas = document.createElement('canvas');
        canvas.style.width = `${window.innerWidth}px`;
        canvas.style.height = `${window.innerHeight}px`;
        canvas.style.position = 'fixed';
        canvas.style.top = '0';
        canvas.style.left = '0';
        document.body.appendChild(canvas);
        UIStore.canvases.set(displayId, canvas);
    }
    canvas.width = frame.codedWidth;
    canvas.height = frame.codedHeight;
    const cxt = canvas.getContext('2d')!;
    cxt.imageSmoothingEnabled = false;
    cxt.drawImage(
        frame,
        0,
        0,
        frame.codedWidth,
        frame.codedHeight,
        0,
        0,
        canvas.width,
        canvas.height
    );
}

export function handleFrameError(displayId: number, error: DOMException) {
    throw error;
}
