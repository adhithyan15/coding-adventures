if command -v xcrun >/dev/null 2>&1; then xcrun swift test; else swift test; fi
# Guard: every Swift lexer/parser _Grammar.swift must match its canonical grammar.
bash ./regen-embedded-grammars.sh --check
