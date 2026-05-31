#!/bin/bash
set -e

SERVICE="discordbot"
START_TIME=$(date +%s.%N)
BUILD_OUTPUT=$(cargo build --release --timings 2>&1)
BUILD_EXIT=$?
END_TIME=$(date +%s.%N)
COMPILE_TIME=$(awk "BEGIN {printf \"%.2f\", $END_TIME - $START_TIME}")
echo "$BUILD_OUTPUT"

if [ $BUILD_EXIT -ne 0 ]; then
    echo "Build failed."
    exit $BUILD_EXIT
fi

PACKAGE_NAME=$(grep '^name *= *' Cargo.toml | head -n1 | cut -d'"' -f2)
PACKAGE_VERSION=$(grep '^version *= *' Cargo.toml | head -n1 | cut -d'"' -f2)
PACKAGE_PATH=$(pwd)
WARNING_COUNT=$(echo "$BUILD_OUTPUT" | grep -c '^warning:' || true)

echo "Compiled $PACKAGE_NAME $PACKAGE_VERSION $PACKAGE_PATH Compile time ${COMPILE_TIME}s Warnings caught $WARNING_COUNT Restarting systemd service for instance $SERVICE"
sudo systemctl restart "$SERVICE"
