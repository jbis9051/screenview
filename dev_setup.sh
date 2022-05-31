#!/usr/bin/env bash
git config core.hooksPath hooks

cd packages/rust || exit
cargo build
cd ../js || exit
yarn install
yarn build
