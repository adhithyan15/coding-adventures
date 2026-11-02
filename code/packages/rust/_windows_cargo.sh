#!/usr/bin/env bash
set -euo pipefail

# Windows cargo wrapper: point cargo at the MSVC linker that vswhere finds.
#
# Which `bash` runs this script depends on PATH, and the two candidates on a
# Windows CI runner spell drive paths differently:
#
#   shell         C:\Program Files (x86)  as a POSIX path
#   ------------  -----------------------------------------
#   Git Bash      /c/Program Files (x86)
#   Cygwin        /cygdrive/c/Program Files (x86)
#
# Cygwin's bash wins whenever ocaml/setup-ocaml has run earlier in the same
# job, so a hard-coded `/c/...` path is "not found" there. Both shells ship
# `cygpath`, which translates a Windows path into whatever spelling the
# running shell expects, so every path below goes through it.

to_posix() {
  cygpath -u "$1"
}

PROGRAM_FILES_X86=$(printenv 'ProgramFiles(x86)' || true)
PROGRAM_FILES_X86=${PROGRAM_FILES_X86:-'C:\Program Files (x86)'}
VSWHERE="$(to_posix "$PROGRAM_FILES_X86")/Microsoft Visual Studio/Installer/vswhere.exe"

if [ ! -f "$VSWHERE" ]; then
  echo "vswhere.exe not found at $VSWHERE" >&2
  exit 1
fi

VS_INSTALL_PATH=$("$VSWHERE" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | tr -d '\r')

if [ -z "$VS_INSTALL_PATH" ]; then
  echo "Unable to locate a Visual Studio installation with C++ tools" >&2
  exit 1
fi

# vswhere answers with a Windows path (C:\Program Files\...); `find` needs
# the running shell's own spelling of it.
LINKER_PATH=$(find "$(to_posix "$VS_INSTALL_PATH")/VC/Tools/MSVC" -path '*/bin/Hostx64/x64/link.exe' | sort | tail -n 1)

if [ -z "$LINKER_PATH" ]; then
  echo "Unable to locate MSVC link.exe under $VS_INSTALL_PATH" >&2
  exit 1
fi

export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=$(cygpath -w "$LINKER_PATH")

cargo "$@"
