#!/bin/bash

set -e

SERVICE="discordbot"
LOG=$(mktemp)

START_TIME=$(date +%s.%N)
cargo build --release --timings >"$LOG" 2>&1 || {
    cat "$LOG"
    rm -f "$LOG"
    exit 1
}
END_TIME=$(date +%s.%N)
COMPILE_TIME=$(awk "BEGIN {printf \"%.2f\", $END_TIME - $START_TIME}")
PACKAGE_NAME=$(grep '^name *= *' Cargo.toml | head -n1 | cut -d'"' -f2)
PACKAGE_VERSION=$(grep '^version *= *' Cargo.toml | head -n1 | cut -d'"' -f2)
PACKAGE_PATH=$(pwd)
WARNING_COUNT=$(grep -c '^warning:' "$LOG" || true)

printf "\nCompiled $PACKAGE_NAME $PACKAGE_VERSION $PACKAGE_PATH\nCompile time ${COMPILE_TIME}s\nWarnings caught $WARNING_COUNT\n\nPlease restart systemd service for instance $SERVICE"

# wanted to do some shell stuff, this wil lbe replced by a command later