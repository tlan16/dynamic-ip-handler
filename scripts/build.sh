#!/usr/bin/env bash
cd "$(dirname "$0")/../" || exit 1
set -euro pipefail

rm -vrf target
docker run --rm -it -v "$(pwd)":/home/rust/src messense/rust-musl-cross:aarch64-musl cargo build --release
docker run --rm -it -v "$(pwd)":/home/rust/src messense/rust-musl-cross:armv7-musleabi cargo build --release
docker run --rm -it -v "$(pwd)":/home/rust/src messense/rust-musl-cross:x86_64-musl cargo build --release

cp -v target/aarch64-unknown-linux-musl/release/dynamic-ip-handler dynamic-ip-handler-aarch64
cp -v target/armv7-unknown-linux-musleabi/release/dynamic-ip-handler dynamic-ip-handler-armv7
cp -v target/x86_64-unknown-linux-musl/release/dynamic-ip-handler dynamic-ip-handler-x86_64

rm -vrf target
