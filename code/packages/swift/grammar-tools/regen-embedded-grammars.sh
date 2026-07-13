#!/usr/bin/env bash
# ============================================================================
# regen-embedded-grammars.sh — (re)generate every Swift package's _Grammar.swift
# ============================================================================
#
# Single source of truth for which Swift lexer/parser packages embed which
# canonical grammar. Run from the repo root:
#
#   ./code/packages/swift/grammar-tools/regen-embedded-grammars.sh          # write
#   ./code/packages/swift/grammar-tools/regen-embedded-grammars.sh --check  # verify
#
# `--check` regenerates into a temp file and diffs against the committed
# _Grammar.swift, failing if any is stale. That is the CI drift guard: it makes
# a grammar edit that forgets to regenerate an un-mergeable red build.
# ============================================================================
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$repo_root"
gt="code/packages/swift/grammar-tools"

# kind | canonical grammar file | label | output _Grammar.swift
manifest=(
  "tokens|code/grammars/algol/algol60.tokens|algol60|code/packages/swift/algol-lexer/Sources/AlgolLexer/_Grammar.swift"
  "grammar|code/grammars/algol/algol60.grammar|algol60|code/packages/swift/algol-parser/Sources/AlgolParser/_Grammar.swift"
  "tokens|code/grammars/dartmouth_basic/dartmouth_basic.tokens|dartmouthBasic|code/packages/swift/dartmouth-basic-lexer/Sources/DartmouthBasicLexer/_Grammar.swift"
  "grammar|code/grammars/dartmouth_basic/dartmouth_basic.grammar|dartmouthBasic|code/packages/swift/dartmouth-basic-parser/Sources/DartmouthBasicParser/_Grammar.swift"
  "tokens|code/grammars/ecmascript/es1.tokens|es1|code/packages/swift/ecmascript-es1-lexer/Sources/EcmascriptES1Lexer/_Grammar.swift"
  "tokens|code/grammars/ecmascript/es3.tokens|es3|code/packages/swift/ecmascript-es3-lexer/Sources/EcmascriptES3Lexer/_Grammar.swift"
  "tokens|code/grammars/ecmascript/es5.tokens|es5|code/packages/swift/ecmascript-es5-lexer/Sources/EcmascriptES5Lexer/_Grammar.swift"
  "tokens|code/grammars/toml/toml.tokens|toml|code/packages/swift/toml-lexer/Sources/TOMLLexer/_Grammar.swift"
  "tokens|code/grammars/xml/xml.tokens|xml|code/packages/swift/xml-lexer/Sources/XMLLexer/_Grammar.swift"
)

check=0
[ "${1:-}" = "--check" ] && check=1
status=0

for entry in "${manifest[@]}"; do
  IFS='|' read -r kind grammar label out <<<"$entry"
  if [ "$check" -eq 1 ]; then
    tmp="$(mktemp)"
    swift run --package-path "$gt" grammar-tools-embed "$kind" "$tmp" EmbeddedGrammar "$label=$grammar" >/dev/null
    # Compare CR-insensitively so a CRLF checkout does not read as drift.
    if ! diff -q <(tr -d '\r' <"$out") <(tr -d '\r' <"$tmp") >/dev/null 2>&1; then
      echo "STALE: $out (regenerate with regen-embedded-grammars.sh)"
      status=1
    fi
    rm -f "$tmp"
  else
    swift run --package-path "$gt" grammar-tools-embed "$kind" "$out" EmbeddedGrammar "$label=$grammar" >/dev/null
    echo "wrote $out"
  fi
done

exit $status
