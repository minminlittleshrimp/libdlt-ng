#!/bin/bash
# Quick benchmark runner - tests all key scenarios

echo "╔═══════════════════════════════════════════════════════════════════════════╗"
echo "║                   DLT BENCHMARK - QUICK VALIDATION                        ║"
echo "╚═══════════════════════════════════════════════════════════════════════════╝"
echo ""

# Build if needed
if [ ! -f target/release/dlt-bench ]; then
    echo "Building dlt-bench..."
    cargo build --release --bin dlt-bench
    echo ""
fi

echo "Running quick benchmark suite..."
echo ""

# Run quick suite
./target/release/dlt-bench -c quick

echo ""
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""
echo "✓ Benchmark complete!"
echo ""
echo "For more detailed benchmarks, try:"
echo "  ./target/release/dlt-bench -c overflow-all       # Compare overflow modes"
echo "  ./target/release/dlt-bench -c concurrency-scale  # Thread scalability"
echo "  ./target/release/dlt-bench -a                    # Full suite (~5 min)"
echo ""
