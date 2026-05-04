#!/usr/bin/env bash
# =============================================================================
# Local pre-push Miri verification for twig-vm
# =============================================================================
#
# Run this before pushing any change that touches `twig-vm` or any of
# its dependencies (`lispy-runtime`, `lang-runtime-core`,
# `interpreter-ir`).  It's the canonical local check that the
# integration seam between safe twig-vm code and unsafe lispy-runtime
# primitives doesn't have UB.
#
# Why local instead of CI?
#
# The twig-vm Miri suite takes ~30-90 min on Linux CI runners.  PR 7
# moved it off the per-PR critical path — `lang-runtime-safety.yml`
# runs lang-runtime-core + lispy-runtime Miri as blocking checks
# (~5 min total), and twig-vm Miri runs as `continue-on-error: true`
# informational + as a nightly regression check.  The canonical
# verification is local, before push.
#
# Usage:
#
#   $ scripts/miri-twig-vm.sh
#
# Exit code 0 = all twig-vm Miri tests pass.
# Exit non-zero = UB detected; do not push without fixing.
#
# Wallclock budget: ~30 min on a fast Mac, ~60 min on a typical
# Linux/WSL development machine.  Run it overnight or in a
# separate terminal during code review.
#
# Requirements:
#   - rustup with nightly toolchain installed
#   - Miri component installed: `rustup +nightly component add miri`
#
# If those are missing, the script prints actionable instructions
# rather than failing opaquely.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGES_RUST="${REPO_ROOT}/code/packages/rust"

# ── Pre-flight: nightly + miri component ─────────────────────────────

if ! command -v rustup >/dev/null 2>&1; then
    echo "Error: rustup not found in PATH."
    echo "Install rustup from https://rustup.rs/ then re-run."
    exit 1
fi

if ! rustup toolchain list | grep -q nightly; then
    echo "Installing Rust nightly toolchain..."
    rustup toolchain install nightly --component miri
fi

if ! rustup +nightly component list --installed 2>/dev/null | grep -q '^miri'; then
    echo "Installing Miri component..."
    rustup +nightly component add miri
fi

# ── Configuration ────────────────────────────────────────────────────

# See `.github/workflows/lang-runtime-safety.yml` for the full
# rationale behind these flags.  Same flags as CI.
export MIRIFLAGS="-Zmiri-ignore-leaks"
export CARGO_INCREMENTAL="0"
export RUST_BACKTRACE="1"

# ── Run ──────────────────────────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════════"
echo "  Running Miri on twig-vm + lispy-runtime + lang-runtime-core"
echo "  Working directory: ${PACKAGES_RUST}"
echo "  MIRIFLAGS: ${MIRIFLAGS}"
echo "  This typically takes 30-90 min.  Press Ctrl-C to abort."
echo "═══════════════════════════════════════════════════════════════════"
echo

cd "${PACKAGES_RUST}"

# Run the fast crates first.  If lang-runtime-core or lispy-runtime
# Miri fails, twig-vm Miri will fail too — surface the smaller
# failure first.
echo "→ Miri lang-runtime-core (fast, ~13s)..."
cargo +nightly miri test -p lang-runtime-core --no-fail-fast

echo
echo "→ Miri lispy-runtime (fast, ~13s)..."
cargo +nightly miri test -p lispy-runtime --no-fail-fast

echo
echo "→ Miri twig-vm (slow, 30-90 min)..."
cargo +nightly miri test -p twig-vm --no-fail-fast

echo
echo "═══════════════════════════════════════════════════════════════════"
echo "  ✅ All Miri checks passed.  Safe to push."
echo "═══════════════════════════════════════════════════════════════════"
