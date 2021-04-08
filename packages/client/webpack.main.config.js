// eslint-disable-next-line @typescript-eslint/no-var-requires
const path = require('path');

module.exports = {
    entry: ['./src/main/main.ts'],
    target: 'electron-main',
    output: {
        path: path.join(__dirname, 'build', 'main'),
        filename: 'main.js',
    },
    resolve: {
        extensions: ['.js', '.ts'],
    },
    module: {
        rules: [
            {
                test: /\.(ts|tsx)$/,
                loader: 'ts-loader',
            },
        ],
    },
};
