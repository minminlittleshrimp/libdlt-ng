#!/bin/bash
# Continuous logging test to detect message loss

echo "=== DLT Continuous Logging Test ==="
echo "This will send 1000 messages and check for gaps"
echo ""

# Kill any existing daemons
killall -9 dlt-daemon 2>/dev/null
sleep 1

# Start our daemon
echo "Starting dlt-daemon..."
./target/release/dlt-daemon &
DAEMON_PID=$!
sleep 2

# Send logs with sequential numbering
echo "Sending 1000 messages..."
./target/release/dlt-example-user -n 1000 -d 5 TestMsg > /tmp/dlt-send.log 2>&1

# Give time for all messages to be processed
sleep 2

echo ""
echo "=== Checking for message gaps ==="

# Start receiver in background and capture output
timeout 5 ./target/release/dlt-receive -a localhost > /tmp/dlt-receive.log 2>&1 &
RECV_PID=$!

# Wait for receiver to finish or timeout
sleep 6

# Analyze the received messages
echo ""
echo "=== Analysis ==="
if [ -f /tmp/dlt-receive.log ]; then
    # Extract message numbers
    grep "TestMsg" /tmp/dlt-receive.log | awk '{print $NF}' | grep -oE '[0-9]+' | sort -n > /tmp/dlt-numbers.txt

    TOTAL=$(wc -l < /tmp/dlt-numbers.txt)
    echo "Total messages received: $TOTAL / 1000"

    # Check for gaps
    echo ""
    echo "Checking for gaps in sequence..."
    python3 << 'EOF'
with open('/tmp/dlt-numbers.txt') as f:
    numbers = [int(line.strip()) for line in f if line.strip()]

if not numbers:
    print("No messages received!")
else:
    gaps = []
    for i in range(len(numbers) - 1):
        diff = numbers[i+1] - numbers[i]
        if diff > 1:
            gaps.append((numbers[i], numbers[i+1]))

    if gaps:
        print(f"Found {len(gaps)} gaps:")
        for start, end in gaps[:10]:  # Show first 10 gaps
            print(f"  Gap: {start} -> {end} (missing {end - start - 1} messages)")
    else:
        print("No gaps found! All messages received in sequence.")

    print(f"\nFirst message: {numbers[0]}")
    print(f"Last message: {numbers[-1]}")
EOF
else
    echo "ERROR: No receiver output found"
fi

# Cleanup
kill $DAEMON_PID 2>/dev/null
echo ""
echo "Test complete. Logs saved to /tmp/dlt-*.log"
