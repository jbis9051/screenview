// God knows why I have to do this
import * as interop from '@screenview/node-interop';

// @ts-ignore
// eslint-disable-next-line @typescript-eslint/no-var-requires
const interop2: typeof interop = window.require('@screenview/node-interop');

export default interop2;
