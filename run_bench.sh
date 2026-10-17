#!/usr/bin/env bash
set -e

# Target your specific subcrate name
SUBCRATE="blink_html"

echo "🧪 Running Base Tree A Architecture..."
cargo bench -p $SUBCRATE --bench dom_comparison -- --save-baseline dom_a

echo "🧪 Running Feature-Flagged Tree B Architecture..."
cargo bench -p $SUBCRATE --bench dom_comparison --features "dense-core" -- --baseline dom_a
