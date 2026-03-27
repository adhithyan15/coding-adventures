#!/usr/bin/env bash
# generate-compiled-grammars.sh — compile all grammars into downstream packages
# =============================================================================
#
# This script is the source of truth for the grammar-to-package mapping.
# Run it from the repository root (or any directory inside the repo) whenever
# a .tokens or .grammar file changes.
#
# What it does
# ------------
#   For every grammar in code/grammars/ it finds all downstream packages across
#   all 5 implemented languages (Python, Go, Ruby, TypeScript, Rust) and calls
#   the appropriate grammar-tools compile command to write _grammar.{ext} into
#   each package's source directory.
#
# Usage
#   scripts/generate-compiled-grammars.sh
#
# Requirements
#   mise (to run language-specific tools)
#   Go installed (for building the Go binary)
#   cargo (for building the Rust binary)
#   node + npm (TypeScript program, already has node_modules)
#
# Output
#   _grammar.py   — in code/packages/python/{name}-{lexer,parser}/src/{pkg}/
#   _grammar.go   — in code/packages/go/{name}-{lexer,parser}/
#   _grammar.rb   — in code/packages/ruby/{name}_{lexer,parser}/lib/coding_adventures/{pkg}/
#   _grammar.ts   — in code/packages/typescript/{name}-{lexer,parser}/src/
#   _grammar.rs   — in code/packages/rust/{name}-{lexer,parser}/src/
#
# Design notes
# ------------
#   - Elixir and Lua have no grammar-dependent packages yet; they are skipped.
#   - The Rust xml-lexer uses xml_rust.tokens (not xml.tokens) because it is
#     optimised for Rust's regex engine.
#   - Go requires a --package flag so the generated file declares the correct
#     package name. The name is derived by stripping hyphens from the directory
#     name (e.g. json-lexer → jsonlexer).
#   - Each compile call is guarded: if the target directory does not exist the
#     command is silently skipped rather than erroring.

set -euo pipefail

# ---------------------------------------------------------------------------
# Locate repo root
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

GRAMMARS="$REPO_ROOT/code/grammars"
PACKAGES="$REPO_ROOT/code/packages"
PROGRAMS="$REPO_ROOT/code/programs"

# ---------------------------------------------------------------------------
# Build language toolchains (build once, use many times)
# ---------------------------------------------------------------------------

echo "=== Building grammar-tools binaries ==="

# Go: build a native binary into a temp directory
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
GO_BINARY="$TMP_DIR/go-grammar-tools"
echo -n "  Go ... "
cd "$PROGRAMS/go/grammar-tools"
go build -o "$GO_BINARY" . 2>&1
echo "OK"

# Rust: build release binary
echo -n "  Rust ... "
cd "$PROGRAMS/rust/grammar-tools"
cargo build --release --quiet 2>&1
RS_BINARY="$PROGRAMS/rust/grammar-tools/target/release/grammar-tools"
echo "OK"

# Python: invoke directly via mise
PY_PROG="$PROGRAMS/python/grammar-tools/main.py"

# Ruby: invoke directly (LANG=en_US.UTF-8 ensures UTF-8 string handling on macOS)
RB_PROG="$PROGRAMS/ruby/grammar-tools/main.rb"

# TypeScript: use vite-node with the wrapper script that bypasses the main-module
# guard (incompatible with vite-node's argv[1] behaviour) and calls the compile
# functions directly.  The vitest.config.ts provides alias resolution for the
# file: dependencies.
TS_DIR="$PROGRAMS/typescript/grammar-tools"
TS_VITE="$TS_DIR/node_modules/.bin/vite-node"
TS_VITE_CFG="$TS_DIR/vitest.config.ts"
TS_WRAPPER="$SCRIPT_DIR/_ts_grammar_compile.ts"

echo ""

# ---------------------------------------------------------------------------
# Helper: derive Go package name from directory name
# e.g. "json-lexer" → "jsonlexer", "typescript-parser" → "typescriptparser"
# ---------------------------------------------------------------------------

go_pkg_name() {
    # Strip hyphens and lower-case; Go convention for these packages
    echo "$1" | tr -d '-'
}

# ---------------------------------------------------------------------------
# Helper: run a compile command and report result
# ---------------------------------------------------------------------------

FAILURES=0

compile() {
    local lang="$1"      # display name for error messages
    shift
    local tool_args=("$@")

    if ! "${tool_args[@]}" 2>&1; then
        echo "  FAILED: ${tool_args[*]}"
        FAILURES=$((FAILURES + 1))
    fi
}

# ---------------------------------------------------------------------------
# Per-language compile functions
# ---------------------------------------------------------------------------

compile_python_tokens() {
    local grammar="$1"    # e.g. "json"
    local tokens="$GRAMMARS/${grammar}.tokens"
    local pkg_dir="${grammar//-/_}_lexer"
    local out_dir="$PACKAGES/python/${grammar}-lexer/src/$pkg_dir"
    [ -d "$out_dir" ] || return 0
    echo "  python: ${grammar}-lexer/_grammar.py"
    compile python \
        mise exec -- python "$PY_PROG" compile-tokens "$tokens" -o "$out_dir/_grammar.py"
}

compile_python_grammar() {
    local grammar="$1"
    local grammar_file="$GRAMMARS/${grammar}.grammar"
    local pkg_dir="${grammar//-/_}_parser"
    local out_dir="$PACKAGES/python/${grammar}-parser/src/$pkg_dir"
    [ -d "$out_dir" ] || return 0
    echo "  python: ${grammar}-parser/_grammar.py"
    compile python \
        mise exec -- python "$PY_PROG" compile-grammar "$grammar_file" -o "$out_dir/_grammar.py"
}

compile_go_tokens() {
    local grammar="$1"
    local force="${2:-}"   # pass "--force" for grammars with lookahead patterns
    local tokens="$GRAMMARS/${grammar}.tokens"
    local pkg_dir="${grammar}-lexer"
    local out_dir="$PACKAGES/go/$pkg_dir"
    [ -d "$out_dir" ] || return 0
    local pkg
    pkg="$(go_pkg_name "$pkg_dir")"
    echo "  go: ${grammar}-lexer/_grammar.go"
    # shellcheck disable=SC2086
    compile go \
        "$GO_BINARY" compile-tokens "$tokens" -o "$out_dir/_grammar.go" -p "$pkg" $force
}

compile_go_grammar() {
    local grammar="$1"
    local force="${2:-}"   # pass "--force" for grammars with lookahead patterns
    local grammar_file="$GRAMMARS/${grammar}.grammar"
    local pkg_dir="${grammar}-parser"
    local out_dir="$PACKAGES/go/$pkg_dir"
    [ -d "$out_dir" ] || return 0
    local pkg
    pkg="$(go_pkg_name "$pkg_dir")"
    echo "  go: ${grammar}-parser/_grammar.go"
    # shellcheck disable=SC2086
    compile go \
        "$GO_BINARY" compile-grammar "$grammar_file" -o "$out_dir/_grammar.go" -p "$pkg" $force
}

compile_ruby_tokens() {
    local grammar="$1"
    local tokens="$GRAMMARS/${grammar}.tokens"
    local pkg_dir="${grammar//-/_}_lexer"
    local out_dir="$PACKAGES/ruby/$pkg_dir/lib/coding_adventures/$pkg_dir"
    [ -d "$out_dir" ] || return 0
    echo "  ruby: ${grammar}_lexer/_grammar.rb"
    compile ruby \
        env LANG=en_US.UTF-8 mise exec -- ruby "$RB_PROG" compile-tokens "$tokens" -o "$out_dir/_grammar.rb"
}

compile_ruby_grammar() {
    local grammar="$1"
    local grammar_file="$GRAMMARS/${grammar}.grammar"
    local pkg_dir="${grammar//-/_}_parser"
    local out_dir="$PACKAGES/ruby/$pkg_dir/lib/coding_adventures/$pkg_dir"
    [ -d "$out_dir" ] || return 0
    echo "  ruby: ${grammar}_parser/_grammar.rb"
    compile ruby \
        env LANG=en_US.UTF-8 mise exec -- ruby "$RB_PROG" compile-grammar "$grammar_file" -o "$out_dir/_grammar.rb"
}

compile_typescript_tokens() {
    local grammar="$1"
    local tokens="$GRAMMARS/${grammar}.tokens"
    local out_dir="$PACKAGES/typescript/${grammar}-lexer/src"
    [ -d "$out_dir" ] || return 0
    echo "  typescript: ${grammar}-lexer/_grammar.ts"
    compile typescript \
        sh -c "cd \"$TS_DIR\" && mise exec -- node \"$TS_VITE\" --config \"$TS_VITE_CFG\" \"$TS_WRAPPER\" tokens \"$tokens\" \"$out_dir/_grammar.ts\""
}

compile_typescript_grammar() {
    local grammar="$1"
    local grammar_file="$GRAMMARS/${grammar}.grammar"
    local out_dir="$PACKAGES/typescript/${grammar}-parser/src"
    [ -d "$out_dir" ] || return 0
    echo "  typescript: ${grammar}-parser/_grammar.ts"
    compile typescript \
        sh -c "cd \"$TS_DIR\" && mise exec -- node \"$TS_VITE\" --config \"$TS_VITE_CFG\" \"$TS_WRAPPER\" grammar \"$grammar_file\" \"$out_dir/_grammar.ts\""
}

compile_rust_tokens() {
    local grammar="$1"
    local tokens_name="${2:-$grammar}"   # allows overriding (e.g. xml_rust)
    local force="${3:-}"                 # pass "--force" for grammars with lookahead patterns
    local tokens="$GRAMMARS/${tokens_name}.tokens"
    local out_dir="$PACKAGES/rust/${grammar}-lexer/src"
    [ -d "$out_dir" ] || return 0
    echo "  rust: ${grammar}-lexer/_grammar.rs"
    # shellcheck disable=SC2086
    compile rust \
        "$RS_BINARY" compile-tokens "$tokens" -o "$out_dir/_grammar.rs" $force
}

compile_rust_grammar() {
    local grammar="$1"
    local force="${2:-}"   # pass "--force" for grammars with lookahead patterns
    local grammar_file="$GRAMMARS/${grammar}.grammar"
    local out_dir="$PACKAGES/rust/${grammar}-parser/src"
    [ -d "$out_dir" ] || return 0
    echo "  rust: ${grammar}-parser/_grammar.rs"
    # shellcheck disable=SC2086
    compile rust \
        "$RS_BINARY" compile-grammar "$grammar_file" -o "$out_dir/_grammar.rs" $force
}

# ---------------------------------------------------------------------------
# All compile calls — one section per grammar
# ---------------------------------------------------------------------------

echo "=== Generating _grammar files ==="
echo ""

# --- css ---
echo "css:"
compile_python_tokens  css
compile_python_grammar css
compile_rust_tokens    css
compile_rust_grammar   css
echo ""

# --- excel ---
# Note: excel.tokens uses lookahead (?!...) patterns unsupported by Go/Rust regex engines.
# The downstream packages handle these at runtime; --force embeds them as literal strings.
echo "excel:"
compile_python_tokens     excel
compile_python_grammar    excel
compile_go_tokens         excel --force
compile_go_grammar        excel --force
compile_ruby_tokens       excel
compile_ruby_grammar      excel
compile_typescript_tokens excel
compile_typescript_grammar excel
compile_rust_tokens       excel excel --force
compile_rust_grammar      excel --force
echo ""

# --- javascript ---
echo "javascript:"
compile_python_tokens     javascript
compile_python_grammar    javascript
compile_go_tokens         javascript
compile_go_grammar        javascript
compile_ruby_tokens       javascript
compile_ruby_grammar      javascript
compile_typescript_tokens javascript
compile_typescript_grammar javascript
compile_rust_tokens       javascript
compile_rust_grammar      javascript
echo ""

# --- json ---
echo "json:"
compile_python_tokens     json
compile_python_grammar    json
compile_go_tokens         json
compile_go_grammar        json
compile_ruby_tokens       json
compile_ruby_grammar      json
compile_typescript_tokens json
compile_typescript_grammar json
compile_rust_tokens       json
compile_rust_grammar      json
echo ""

# --- lattice ---
echo "lattice:"
compile_python_tokens     lattice
compile_python_grammar    lattice
compile_go_tokens         lattice
compile_go_grammar        lattice
compile_ruby_tokens       lattice
compile_ruby_grammar      lattice
compile_typescript_tokens lattice
compile_typescript_grammar lattice
compile_rust_tokens       lattice
compile_rust_grammar      lattice
echo ""

# --- lisp ---
echo "lisp:"
compile_python_tokens  lisp
compile_python_grammar lisp
compile_rust_tokens    lisp
compile_rust_grammar   lisp
echo ""

# --- python ---
echo "python:"
compile_python_tokens     python
compile_python_grammar    python
compile_go_tokens         python
compile_go_grammar        python
compile_ruby_tokens       python
compile_ruby_grammar      python
compile_typescript_tokens python
compile_typescript_grammar python
# (no Rust python-lexer/parser packages)
echo ""

# --- ruby ---
echo "ruby:"
compile_python_tokens     ruby
compile_python_grammar    ruby
compile_go_tokens         ruby
compile_go_grammar        ruby
compile_ruby_tokens       ruby
compile_ruby_grammar      ruby
compile_typescript_tokens ruby
compile_typescript_grammar ruby
compile_rust_tokens       ruby
compile_rust_grammar      ruby
echo ""

# --- sql ---
echo "sql:"
compile_python_tokens     sql
compile_python_grammar    sql
compile_go_tokens         sql
compile_go_grammar        sql
compile_ruby_tokens       sql
compile_ruby_grammar      sql
compile_typescript_tokens sql
compile_typescript_grammar sql
compile_rust_tokens       sql
compile_rust_grammar      sql
echo ""

# --- starlark ---
echo "starlark:"
compile_python_tokens     starlark
compile_python_grammar    starlark
compile_go_tokens         starlark
compile_go_grammar        starlark
compile_ruby_tokens       starlark
compile_ruby_grammar      starlark
compile_typescript_tokens starlark
compile_typescript_grammar starlark
compile_rust_tokens       starlark
compile_rust_grammar      starlark
echo ""

# --- toml ---
echo "toml:"
compile_python_tokens     toml
compile_python_grammar    toml
compile_go_tokens         toml
compile_go_grammar        toml
compile_ruby_tokens       toml
compile_ruby_grammar      toml
compile_typescript_tokens toml
compile_typescript_grammar toml
compile_rust_tokens       toml
compile_rust_grammar      toml
echo ""

# --- typescript ---
echo "typescript:"
compile_python_tokens     typescript
compile_python_grammar    typescript
compile_go_tokens         typescript
compile_go_grammar        typescript
compile_ruby_tokens       typescript
compile_ruby_grammar      typescript
compile_typescript_tokens typescript
compile_typescript_grammar typescript
compile_rust_tokens       typescript
compile_rust_grammar      typescript
echo ""

# --- verilog ---
echo "verilog:"
compile_python_tokens     verilog
compile_python_grammar    verilog
compile_go_tokens         verilog
compile_go_grammar        verilog
compile_ruby_tokens       verilog
compile_ruby_grammar      verilog
compile_typescript_tokens verilog
compile_typescript_grammar verilog
compile_rust_tokens       verilog
compile_rust_grammar      verilog
echo ""

# --- vhdl ---
echo "vhdl:"
compile_python_tokens     vhdl
compile_python_grammar    vhdl
compile_go_tokens         vhdl
compile_go_grammar        vhdl
compile_ruby_tokens       vhdl
compile_ruby_grammar      vhdl
compile_typescript_tokens vhdl
compile_typescript_grammar vhdl
compile_rust_tokens       vhdl
compile_rust_grammar      vhdl
echo ""

# --- xml (lexer only; all 5 languages) ---
# Note: xml.tokens uses lookahead (?!...) patterns unsupported by Go/Rust regex engines.
# The downstream packages handle these at runtime; --force embeds them as literal strings.
# Rust xml-lexer uses xml_rust.tokens (optimised for Rust's regex engine) — no lookaheads.
echo "xml:"
compile_python_tokens     xml
compile_go_tokens         xml --force
compile_ruby_tokens       xml
compile_typescript_tokens xml
compile_rust_tokens       xml xml_rust
echo ""

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------

echo "==================================="
if [ "$FAILURES" -eq 0 ]; then
    echo "All _grammar files generated successfully."
else
    echo "FAILURES: $FAILURES compile command(s) failed."
    exit 1
fi
