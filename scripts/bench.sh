#!/usr/bin/env bash
# Runs the engine benchmarks (the ignored bench_ tests in core/tests/engine.rs) in release and
# prints one BENCH line each. Two engines talk over localhost in one process, writing to the
# temp folder (set TMPDIR to measure another disk).
#
#   scripts/bench.sh                  # all of them
#   scripts/bench.sh many_small       # only those whose name contains "many_small"
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --release -p nhako-core --test engine --no-run 2>/dev/null
# Capped at 3 GB where systemd can, so a runaway run is stopped instead of the whole computer.
cap=()
if command -v systemd-run >/dev/null && systemd-run --user --scope -q true 2>/dev/null; then
  cap=(systemd-run --user --scope -q -p MemoryMax=3G -p MemorySwapMax=0)
fi
for name in one_big_file many_small_files big_offer resume_after_drop; do
  [[ -z "${1:-}" || "$name" == *"$1"* ]] || continue
  "${cap[@]}" cargo test -q --release -p nhako-core --test engine "bench_$name" -- --ignored --nocapture 2>&1 \
    | grep -E '^BENCH|panicked' || echo "BENCH $name: stopped (out of memory or crashed)"
done
