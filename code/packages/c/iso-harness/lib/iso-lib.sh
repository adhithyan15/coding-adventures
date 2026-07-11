#!/bin/sh
# iso-lib.sh — a portable, multi-compiler "pure ISO C/C++" build harness.
#
# ─────────────────────────────────────────────────────────────────────────────
# WHY THIS EXISTS
# ─────────────────────────────────────────────────────────────────────────────
# A program can compile cleanly on your machine and still be quietly wrong: it
# may lean on a *compiler extension* that the ISO C/C++ standard does not
# guarantee. GCC, Clang, and MSVC each ship their own extensions, and each
# accepts a slightly different dialect by default. The only reliable way to know
# a translation unit is *portable, standard* C/C++ is to compile it with several
# independent compilers, each told to reject everything the standard does not
# require.
#
# This library does exactly that. Given some source files, it compiles them with
# EVERY compiler it can find on this machine, using the strictest standards-
# conformance flags each compiler offers:
#
#     GCC / Clang :  -std=c17 / -std=c++17  -pedantic-errors  -Wall -Wextra -Werror
#     MSVC        :  /std:c17 / /std:c++17  /permissive-      /W4 /WX     (see iso-lib.ps1)
#
# `-pedantic-errors` (GCC/Clang) and `/permissive-` (MSVC) are the switches that
# actually turn "uses a non-ISO extension" into a hard error. `-Werror` / `/WX`
# make every remaining diagnostic fatal so nothing slips through as a warning.
#
# ─────────────────────────────────────────────────────────────────────────────
# COVERAGE IS ACHIEVED ACROSS THE CI MATRIX, NOT ON ONE MACHINE
# ─────────────────────────────────────────────────────────────────────────────
# No single machine can host all three compilers: a real GCC lives only on
# Linux, and a real MSVC (`cl.exe`) lives only on Windows (macOS's `gcc` is a
# Clang shim). So the repo's CI runs this harness on all three OSes and lets
# each contribute the compilers only it can host:
#
#     ubuntu  → gcc + clang     windows → cl (+ clang-cl)     macOS → Apple clang
#
# This library therefore compiles with *whatever compilers are present* and
# fails if it finds none. To make a guarantee firm rather than best-effort — for
# example "on ubuntu, BOTH gcc AND clang must have run" — set ISO_REQUIRE (see
# below) and the harness will fail if a required compiler is missing.
#
# ─────────────────────────────────────────────────────────────────────────────
# PUBLIC INTERFACE (source this file, then call these)
# ─────────────────────────────────────────────────────────────────────────────
#   iso_compilers   <c|cpp>                 → echo the space-separated list of
#                                             present compilers for the language.
#   iso_build_and_run <c|cpp> <name> <src…> → compile <src…> into one executable
#                                             with every present compiler and run
#                                             it. Fails if any compile/run fails,
#                                             if no compiler is found, or if an
#                                             ISO_REQUIRE compiler is missing.
#   iso_expect_compile_fail <c|cpp> <src>   → assert <src> is REJECTED by every
#                                             present compiler under the strict
#                                             flags (a negative conformance test).
#
# ENVIRONMENT KNOBS (all optional):
#   ISO_REQUIRE   space-separated compiler command names that MUST be present,
#                 e.g. ISO_REQUIRE="gcc clang". Missing any → hard failure.
#   ISO_INCLUDE   space-separated include directories; each becomes -I<dir>.
#   ISO_CSTD      override the C standard (default: c17).
#   ISO_CXXSTD    override the C++ standard (default: c++17).
#   ISO_BUILD_DIR output directory (default: _build). NEVER "build" — on
#                 case-insensitive filesystems it collides with the BUILD script.
#
# POSIX-sh only: no bashisms (no arrays, no `local`, no `[[ ]]`), because BUILD
# scripts on CI run under dash. Lists are space-separated strings.
# ─────────────────────────────────────────────────────────────────────────────

# ── Configuration defaults ───────────────────────────────────────────────────
ISO_CSTD="${ISO_CSTD:-c17}"
ISO_CXXSTD="${ISO_CXXSTD:-c++17}"
ISO_BUILD_DIR="${ISO_BUILD_DIR:-_build}"

# iso__strict_flags LANG → echo the strict conformance flags for the language.
# Shared by GCC and Clang (their flag spelling is identical here).
iso__strict_flags() {
    _warn="-pedantic-errors -Wall -Wextra -Werror"
    if [ "$1" = "cpp" ]; then
        echo "-std=$ISO_CXXSTD $_warn"
    else
        echo "-std=$ISO_CSTD $_warn"
    fi
}

# iso__include_flags → echo -I flags built from ISO_INCLUDE (may be empty).
iso__include_flags() {
    _inc=""
    for _dir in $ISO_INCLUDE; do
        _inc="$_inc -I$_dir"
    done
    echo "$_inc"
}

# iso_compilers LANG → echo the present compilers for LANG, in a stable order.
# We probe by canonical command name so we do not accidentally run the same
# compiler twice under two aliases. `command -v` is POSIX and honors PATH.
iso_compilers() {
    if [ "$1" = "cpp" ]; then
        _candidates="g++ clang++"
    else
        _candidates="gcc clang"
    fi
    _found=""
    for _cc in $_candidates; do
        if command -v "$_cc" >/dev/null 2>&1; then
            _found="$_found $_cc"
        fi
    done
    # Trim the leading space for tidy output.
    echo "$_found" | sed 's/^ *//'
}

# iso__require_check LANG → verify every ISO_REQUIRE compiler is present.
# Returns 0 if satisfied (or if ISO_REQUIRE is unset), 1 otherwise.
iso__require_check() {
    [ -n "$ISO_REQUIRE" ] || return 0
    _missing=""
    for _req in $ISO_REQUIRE; do
        if ! command -v "$_req" >/dev/null 2>&1; then
            _missing="$_missing $_req"
        fi
    done
    if [ -n "$_missing" ]; then
        echo "iso-harness: ERROR required compiler(s) not found:$_missing" >&2
        echo "iso-harness: (ISO_REQUIRE=\"$ISO_REQUIRE\")" >&2
        return 1
    fi
    return 0
}

# iso_build_and_run LANG NAME SRC... → the workhorse.
# Compiles SRC... into one executable with each present compiler and runs it.
iso_build_and_run() {
    _lang="$1"; _name="$2"; shift 2
    _srcs="$*"

    iso__require_check || return 1

    _compilers=$(iso_compilers "$_lang")
    if [ -z "$_compilers" ]; then
        echo "iso-harness: ERROR no $_lang compiler found (need gcc/g++ or clang/clang++)" >&2
        return 1
    fi

    _flags="$(iso__strict_flags "$_lang") $(iso__include_flags)"
    mkdir -p "$ISO_BUILD_DIR"

    _fail=0
    for _cc in $_compilers; do
        _out="$ISO_BUILD_DIR/${_cc}-${_name}"
        echo "iso-harness: [$_cc] compiling $_name"
        # shellcheck disable=SC2086  # word-splitting of $_flags/$_srcs is intended
        if ! "$_cc" $_flags $_srcs -o "$_out"; then
            echo "iso-harness: [$_cc] COMPILE FAILED for $_name" >&2
            _fail=1
            continue
        fi
        echo "iso-harness: [$_cc] running $_name"
        if ! "$_out"; then
            echo "iso-harness: [$_cc] RUNTIME FAILURE for $_name" >&2
            _fail=1
        fi
    done
    return $_fail
}

# iso_expect_compile_fail LANG SRC → negative conformance test.
# Every present compiler must REJECT SRC under the strict flags. If any compiler
# accepts it, the extension slipped past our flags and we fail loudly — this is
# how the harness proves it is actually enforcing ISO conformance.
iso_expect_compile_fail() {
    _lang="$1"; _src="$2"

    iso__require_check || return 1

    _compilers=$(iso_compilers "$_lang")
    if [ -z "$_compilers" ]; then
        echo "iso-harness: ERROR no $_lang compiler found for negative test" >&2
        return 1
    fi

    _flags="$(iso__strict_flags "$_lang") $(iso__include_flags)"
    mkdir -p "$ISO_BUILD_DIR"

    _fail=0
    for _cc in $_compilers; do
        echo "iso-harness: [$_cc] expecting REJECTION of $_src"
        # shellcheck disable=SC2086
        if "$_cc" $_flags -fsyntax-only "$_src" >/dev/null 2>&1; then
            echo "iso-harness: [$_cc] ACCEPTED non-ISO source $_src — strict flags are not enforcing conformance!" >&2
            _fail=1
        else
            echo "iso-harness: [$_cc] correctly rejected $_src"
        fi
    done
    return $_fail
}
