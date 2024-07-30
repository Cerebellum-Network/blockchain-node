#!/usr/bin/env bash

../target/release/cere \
--base-path /tmp/charlie \
--chain local \
--charlie \
--port 30334 \
--rpc-port 9947 \
--telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
--validator
