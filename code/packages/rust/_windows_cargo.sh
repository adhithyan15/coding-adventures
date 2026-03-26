#!/usr/bin/env bash
set -euo pipefail

VSWHERE="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"

if [ ! -x "$VSWHERE" ]; then
  echo "vswhere.exe not found at $VSWHERE" >&2
  exit 1
fi

VS_INSTALL_PATH=$("$VSWHERE" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | tr -d '\r')

if [ -z "$VS_INSTALL_PATH" ]; then
  echo "Unable to locate a Visual Studio installation with C++ tools" >&2
  exit 1
fi

LINKER_PATH=$(find "$VS_INSTALL_PATH/VC/Tools/MSVC" -path '*/bin/Hostx64/x64/link.exe' | sort | tail -n 1)

if [ -z "$LINKER_PATH" ]; then
  echo "Unable to locate MSVC link.exe under $VS_INSTALL_PATH" >&2
  exit 1
fi

export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=$(cygpath -w "$LINKER_PATH")

cargo "$@"
