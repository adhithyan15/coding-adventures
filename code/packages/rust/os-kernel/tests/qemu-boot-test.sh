#!/usr/bin/env bash
set -euo pipefail

ARCH="${1:-}"
if [[ -z "$ARCH" ]]; then
  echo "usage: $0 <x86_64|aarch64>" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "$0")/../../../../.." && pwd)"
PKG_DIR="$ROOT/code/packages/rust"
TARGET_DIR="$PKG_DIR/target"
ESP_DIR="$TARGET_DIR/qemu-esp-$ARCH"
LOG_FILE="$TARGET_DIR/qemu-$ARCH.log"
QEMU_SHARE_DIR="${QEMU_SHARE_DIR:-}"

find_qemu_share_dir() {
  if [[ -n "$QEMU_SHARE_DIR" && -d "$QEMU_SHARE_DIR" ]]; then
    printf '%s\n' "$QEMU_SHARE_DIR"
    return 0
  fi

  if command -v brew >/dev/null 2>&1; then
    local prefix
    prefix="$(brew --prefix qemu 2>/dev/null || true)"
    if [[ -n "$prefix" && -d "$prefix/share/qemu" ]]; then
      printf '%s\n' "$prefix/share/qemu"
      return 0
    fi
  fi

  for candidate in /opt/homebrew/share/qemu /usr/local/share/qemu /usr/share/qemu; do
    if [[ -d "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  echo "unable to locate QEMU firmware directory" >&2
  exit 1
}

find_qemu_firmware() {
  local firmware_name="$1"

  if [[ -n "$QEMU_SHARE_DIR" && -f "$QEMU_SHARE_DIR/$firmware_name" ]]; then
    printf '%s\n' "$QEMU_SHARE_DIR/$firmware_name"
    return 0
  fi

  local share_dir
  share_dir="$(find_qemu_share_dir)"
  if [[ -f "$share_dir/$firmware_name" ]]; then
    printf '%s\n' "$share_dir/$firmware_name"
    return 0
  fi

  for candidate in \
    "/opt/homebrew/opt/qemu/share/qemu/$firmware_name" \
    "/opt/homebrew/share/qemu/$firmware_name" \
    "/usr/local/share/qemu/$firmware_name" \
    "/usr/share/qemu/$firmware_name"; do
    if [[ -f "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  echo "unable to locate QEMU firmware file: $firmware_name" >&2
  exit 1
}

run_with_timeout() {
  local timeout_seconds="$1"
  shift

  /usr/bin/python3 - "$timeout_seconds" "$LOG_FILE" "$@" <<'PY'
import subprocess
import sys

timeout_seconds = float(sys.argv[1])
log_path = sys.argv[2]
cmd = sys.argv[3:]

with open(log_path, "w", encoding="utf-8") as log_file:
    try:
        completed = subprocess.run(
            cmd,
            stdout=log_file,
            stderr=subprocess.STDOUT,
            timeout=timeout_seconds,
            check=False,
        )
        raise SystemExit(completed.returncode)
    except subprocess.TimeoutExpired:
        raise SystemExit(124)
PY
}

case "$ARCH" in
  x86_64)
    RUST_TARGET="x86_64-unknown-uefi"
    EFI_NAME="BOOTX64.EFI"
    QEMU_BIN="qemu-system-x86_64"
    FIRMWARE="$(find_qemu_firmware edk2-x86_64-code.fd)"
    QEMU_ARGS=(-machine q35 -m 1024 -nographic -monitor none)
    FIRMWARE_ARGS=(-drive if=pflash,format=raw,readonly=on,file="$FIRMWARE")
    ESP_DEVICE_ARGS=(-drive if=virtio,format=raw,file=fat:rw:"$ESP_DIR")
    ;;
  aarch64)
    RUST_TARGET="aarch64-unknown-uefi"
    EFI_NAME="BOOTAA64.EFI"
    QEMU_BIN="qemu-system-aarch64"
    FIRMWARE="$(find_qemu_firmware edk2-aarch64-code.fd)"
    QEMU_ARGS=(-machine virt -cpu cortex-a72 -m 1024 -nographic -monitor none)
    FIRMWARE_ARGS=(-bios "$FIRMWARE")
    ESP_DEVICE_ARGS=(-drive if=none,format=raw,file=fat:rw:"$ESP_DIR",id=esp -device virtio-blk-device,drive=esp)
    ;;
  *)
    echo "unsupported arch: $ARCH" >&2
    exit 2
    ;;
esac

if ! command -v "$QEMU_BIN" >/dev/null 2>&1; then
  echo "missing QEMU binary: $QEMU_BIN" >&2
  exit 1
fi

if [[ ! -f "$FIRMWARE" ]]; then
  echo "missing QEMU firmware: $FIRMWARE" >&2
  exit 1
fi

~/.cargo/bin/rustup target add "$RUST_TARGET" >/dev/null
~/.cargo/bin/cargo build -p os-kernel --target "$RUST_TARGET" --manifest-path "$PKG_DIR/Cargo.toml" >/dev/null

rm -rf "$ESP_DIR"
mkdir -p "$ESP_DIR/EFI/BOOT"
cp "$TARGET_DIR/$RUST_TARGET/debug/os-kernel.efi" "$ESP_DIR/EFI/BOOT/$EFI_NAME"

rm -f "$LOG_FILE"
set +e
run_with_timeout 8 "$QEMU_BIN" "${QEMU_ARGS[@]}" \
  "${FIRMWARE_ARGS[@]}" \
  "${ESP_DEVICE_ARGS[@]}" \
  -serial stdio
STATUS=$?
set -e

if [[ $STATUS -ne 0 && $STATUS -ne 124 ]]; then
  cat "$LOG_FILE" >&2
  exit $STATUS
fi

grep -q "kernel: booted" "$LOG_FILE"
grep -q "kernel: running" "$LOG_FILE"
