#!/bin/sh
# Regenerate oracle/expected.sexp with the pinned Lean toolchain (see
# code/learning/lean/krn-s0-spike/lean-toolchain).  Lean is an oracle only: nothing in the
# build or the tests runs it; the output is committed and reviewed.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
lean=${LEAN:-$HOME/.elan/toolchains/leanprover--lean4---v4.35.0/bin/lean}
PATH=$(dirname "$lean"):$PATH
export PATH
"$lean" --version
"$lean" --run "$here/dump.lean" > "$here/expected.sexp"
echo "wrote $here/expected.sexp"
