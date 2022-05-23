#!/usr/bin/env bash
git config core.hooksPath hooks

cd packages/rust || exit
yarn install
cargo build
yarn build
yarn link
cd ../js || exit
yarn link "node-interop"
yarn install
yarn build
