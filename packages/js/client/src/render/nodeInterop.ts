// God knows why I have to do this
import * as interop from 'node-interop';

// @ts-ignore
// eslint-disable-next-line @typescript-eslint/no-var-requires
const interop2: typeof interop = window.require('node-interop');

export default interop2;
