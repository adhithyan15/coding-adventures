#!/bin/sh
# platform-lib.sh — a portable, multi-compiler build harness for OS-dependent
# C/C++ (the non-pure-ISO sibling of iso-harness).
#
# ─────────────────────────────────────────────────────────────────────────────
# WHY THIS EXISTS
# ─────────────────────────────────────────────────────────────────────────────
# iso-harness proves a translation unit is *pure ISO* C/C++ by compiling it with
# every available compiler under `-pedantic-errors` / `/permissive-`. That is
# exactly what we want for computation (math, crypto, codecs, data structures).
#
# But some things fundamentally require the operating system — threads, real
# clocks, filesystem enumeration, sockets, event loops, dynamic loading — and
# there is no pure-ISO way to express them. POSIX and Win32 headers, and idioms
# like `dlsym`'s `void*`→function-pointer cast, are legitimately *not* strict
# ISO. This harness is for that code:
#
#   * it still compiles with EVERY compiler present and still runs the result,
#     so portability across GCC/Clang (and MSVC via platform-lib.ps1) is proven;
#   * it keeps `-Wall -Wextra -Werror` so real bugs remain fatal;
#   * but it DROPS `-pedantic-errors`, because the code deliberately talks to the
#     OS.
#
# Per-OS *source selection* is done by the build-tool, not here: a platform
# package ships BUILD_mac / BUILD_linux / BUILD_windows (each a one-line
# `sh tools/run.sh`), so on any given OS only that OS's backend is compiled and
# this harness never globs or `#if`-picks source files.
#
# ─────────────────────────────────────────────────────────────────────────────
# PUBLIC INTERFACE (source this file, then call these)
# ─────────────────────────────────────────────────────────────────────────────
#   platform_os                                → echo mac | linux | windows | other
#   platform_compilers   <c|cpp>               → echo present compilers for the language
#   platform_build_and_run <c|cpp> <name> <src…>
#                                              → compile <src…> into one executable
#                                                with every present compiler
#                                                (linking $PLATFORM_LIBS) and run it.
#
# ENVIRONMENT KNOBS (all optional):
#   PLATFORM_REQUIRE   space-separated compiler command names that MUST be present
#                      (e.g. "gcc clang"); missing any → hard failure.
#   PLATFORM_INCLUDE   space-separated include directories; each becomes -I<dir>.
#                      (Point this at iso-harness/include to reuse iso_test.h.)
#   PLATFORM_LIBS      extra link tokens appended after the sources, e.g.
#                      "-pthread -ldl". These must be OS-provided libraries only.
#   PLATFORM_DEFINES   space-separated preprocessor defines; each becomes -D<def>,
#                      e.g. "_POSIX_C_SOURCE=200809L".
#   PLATFORM_CSTD      override the C standard (default: c17).
#   PLATFORM_CXXSTD    override the C++ standard (default: c++17).
#   PLATFORM_BUILD_DIR output directory (default: _build). NEVER "build".
#
# POSIX-sh only: no bashisms (no arrays, no `local`, no `[[ ]]`) — BUILD scripts
# run under dash on CI. Lists are space-separated strings.
# ─────────────────────────────────────────────────────────────────────────────

# ── Configuration defaults ───────────────────────────────────────────────────
PLATFORM_CSTD="${PLATFORM_CSTD:-c17}"
PLATFORM_CXXSTD="${PLATFORM_CXXSTD:-c++17}"
PLATFORM_BUILD_DIR="${PLATFORM_BUILD_DIR:-_build}"

# platform_os → echo a short OS tag. Uses `uname` (POSIX); anything non-Darwin/
# Linux (including MSYS/Cygwin shells on Windows) is reported specifically or as
# "other". The Windows/MSVC path normally runs platform-lib.ps1, not this file.
platform_os() {
    case "$(uname -s 2>/dev/null)" in
        Darwin) echo "mac" ;;
        Linux) echo "linux" ;;
        MINGW* | MSYS* | CYGWIN*) echo "windows" ;;
        *) echo "other" ;;
    esac
}

# platform__flags LANG → echo the compile flags for the language. Strict
# warnings-as-errors, but NOT -pedantic-errors (this is OS-dependent code).
platform__flags() {
    _warn="-Wall -Wextra -Werror"
    _def=""
    for _d in $PLATFORM_DEFINES; do
        _def="$_def -D$_d"
    done
    _inc=""
    for _dir in $PLATFORM_INCLUDE; do
        _inc="$_inc -I$_dir"
    done
    if [ "$1" = "cpp" ]; then
        echo "-std=$PLATFORM_CXXSTD $_warn$_def$_inc"
    else
        echo "-std=$PLATFORM_CSTD $_warn$_def$_inc"
    fi
}

# platform_compilers LANG → echo the present compilers for LANG, in a stable
# order, probing by canonical command name so the same compiler is not run twice.
platform_compilers() {
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
    echo "$_found" | sed 's/^ *//'
}

# platform__require_check → verify every PLATFORM_REQUIRE compiler is present.
platform__require_check() {
    [ -n "$PLATFORM_REQUIRE" ] || return 0
    _missing=""
    for _req in $PLATFORM_REQUIRE; do
        if ! command -v "$_req" >/dev/null 2>&1; then
            _missing="$_missing $_req"
        fi
    done
    if [ -n "$_missing" ]; then
        echo "platform-harness: ERROR required compiler(s) not found:$_missing" >&2
        echo "platform-harness: (PLATFORM_REQUIRE=\"$PLATFORM_REQUIRE\")" >&2
        return 1
    fi
    return 0
}

# platform_build_and_run LANG NAME SRC... → the workhorse.
# Compiles SRC... into one executable with each present compiler, linking
# $PLATFORM_LIBS, and runs it. Fails if any compile/run fails, if no compiler is
# found, or if a PLATFORM_REQUIRE compiler is missing.
platform_build_and_run() {
    _lang="$1"; _name="$2"; shift 2
    _srcs="$*"

    platform__require_check || return 1

    _compilers=$(platform_compilers "$_lang")
    if [ -z "$_compilers" ]; then
        echo "platform-harness: ERROR no $_lang compiler found (need gcc/g++ or clang/clang++)" >&2
        return 1
    fi

    _flags="$(platform__flags "$_lang")"
    mkdir -p "$PLATFORM_BUILD_DIR"

    _fail=0
    for _cc in $_compilers; do
        _out="$PLATFORM_BUILD_DIR/${_cc}-${_name}"
        echo "platform-harness: [$_cc] ($(platform_os)) compiling $_name"
        # shellcheck disable=SC2086  # word-splitting of flags/srcs/libs is intended
        if ! "$_cc" $_flags $_srcs $PLATFORM_LIBS -o "$_out"; then
            echo "platform-harness: [$_cc] COMPILE FAILED for $_name" >&2
            _fail=1
            continue
        fi
        echo "platform-harness: [$_cc] running $_name"
        if ! "$_out"; then
            echo "platform-harness: [$_cc] RUNTIME FAILURE for $_name" >&2
            _fail=1
        fi
    done
    return $_fail
}
