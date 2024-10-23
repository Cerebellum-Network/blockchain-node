#!/usr/bin/env bash

../target/release/cere \
--base-path /tmp/alice \
--chain local \
--alice \
--port 30333 \
--rpc-port 9945 \
--telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
--validator
