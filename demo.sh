#!/bin/bash
# Demo script for libdlt-ng - matching old DLT daemon behavior

echo "=== DLT Demo Setup ==="
echo ""
echo "Step 1: Starting DLT Daemon..."
echo "----------------------------------------"

# Kill any existing daemon
pkill -9 dlt-daemon 2>/dev/null

# Start daemon in background
./target/release/dlt-daemon &
DAEMON_PID=$!
sleep 1

echo "Daemon started (PID: $DAEMON_PID)"
echo ""
echo "Step 2: Running dlt-example-user to send 10 log messages..."
echo "----------------------------------------"

./target/release/dlt-example-user -n 10 "Hello from libdlt-ng" &
USER_PID=$!

echo ""
echo "Step 3: Connecting dlt-receive client to view logs..."
echo "----------------------------------------"
sleep 1

# Run receiver for 3 seconds then stop
timeout 3s ./target/release/dlt-receive -a 127.0.0.1

echo ""
echo "----------------------------------------"
echo "Demo complete!"
echo ""
echo "Cleaning up..."
kill $DAEMON_PID 2>/dev/null
wait $USER_PID 2>/dev/null

echo "Done."
