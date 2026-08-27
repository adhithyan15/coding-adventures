set -e

(cd ../pixel-container && npm ci --quiet)
(cd ../paint-instructions && npm ci --quiet)
(cd ../paint-vm && npm ci --quiet)
(cd ../paint-vm-svg && npm ci --quiet)
npm ci --quiet
# HL-C82: the grammar-slot inventory and Spanish cell ledger are generated.
# Fail the build if the committed files drift from the generator, the same
# contract the check:books / check:modality targets already enforce.
# The Python Launcher is the dependable Python 3 entry point on Windows;
# `python3` may only be the Microsoft Store alias and fail when invoked. Keep
# this on one line because the repository build tool executes each BUILD line
# as an independent `sh -c` command.
if command -v python3 >/dev/null 2>&1 && python3 -c 'import sys; raise SystemExit(0 if sys.version_info.major == 3 else 1)' >/dev/null 2>&1; then python3 ../../../learning/human-languages/data/generate_grammar_cells.py --check; elif command -v py >/dev/null 2>&1 && py -3 -c 'import sys; raise SystemExit(0 if sys.version_info.major == 3 else 1)' >/dev/null 2>&1; then py -3 ../../../learning/human-languages/data/generate_grammar_cells.py --check; elif command -v python >/dev/null 2>&1 && python -c 'import sys; raise SystemExit(0 if sys.version_info.major == 3 else 1)' >/dev/null 2>&1; then python ../../../learning/human-languages/data/generate_grammar_cells.py --check; else echo "human-language-data BUILD requires Python 3" >&2; exit 127; fi
npx vitest run --coverage
