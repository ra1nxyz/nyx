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

echo "Compiled $PACKAGE_NAME $PACKAGE_VERSION $PACKAGE_PATH \n Compile time ${COMPILE_TIME}s \n Warnings caught $WARNING_COUNT \n Please restart systemd service for instance $SERVICE"
