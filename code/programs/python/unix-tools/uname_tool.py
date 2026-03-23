"""uname — print system information.

=== What This Program Does ===

This is a reimplementation of the GNU ``uname`` utility. It prints
various pieces of system information: kernel name, hostname, kernel
release, kernel version, machine architecture, and more.

=== System Information Fields ===

``uname`` can display eight different pieces of information, each
controlled by its own flag:

- ``-s`` (kernel name): The operating system kernel name (e.g., "Linux",
  "Darwin"). This is the default if no flags are given.
- ``-n`` (nodename): The network hostname of the machine.
- ``-r`` (kernel release): The kernel version string (e.g., "5.15.0-76-generic").
- ``-v`` (kernel version): The kernel build information (date, build number).
- ``-m`` (machine): The hardware architecture (e.g., "x86_64", "arm64").
- ``-p`` (processor): The processor type (non-portable, may say "unknown").
- ``-i`` (hardware platform): The hardware platform (non-portable).
- ``-o`` (operating system): The OS name (e.g., "GNU/Linux", "Darwin").

=== The -a Flag ===

``-a`` prints all fields in a fixed order, separated by spaces::

    $ uname -a
    Linux myhost 5.15.0 #1 SMP ... x86_64 x86_64 x86_64 GNU/Linux

=== Implementation Notes ===

We use Python's ``platform`` module, which wraps C library calls like
``uname(2)`` on Unix systems. On Windows, some fields may differ from
their Unix equivalents.

=== CLI Builder Integration ===

The entire CLI is defined in ``uname.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import platform
import socket
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "uname.json")


# ---------------------------------------------------------------------------
# Business logic: get_system_info
# ---------------------------------------------------------------------------


def get_system_info() -> dict[str, str]:
    """Gather system information using the ``platform`` module.

    Returns a dictionary with all eight uname fields. Each field is
    a string. On platforms where a field is unavailable, we return
    "unknown".

    The fields are:

    - ``kernel_name``: The OS kernel name (``platform.system()``).
    - ``nodename``: The network hostname (``platform.node()``).
    - ``kernel_release``: The kernel release string (``platform.release()``).
    - ``kernel_version``: The kernel version string (``platform.version()``).
    - ``machine``: The machine hardware name (``platform.machine()``).
    - ``processor``: The processor type (``platform.processor()``).
    - ``hardware_platform``: Same as machine on most systems.
    - ``operating_system``: A human-readable OS name.

    Returns:
        A dictionary mapping field names to their values.
    """
    kernel_name = platform.system() or "unknown"
    nodename = socket.gethostname() or platform.node() or "unknown"
    kernel_release = platform.release() or "unknown"
    kernel_version = platform.version() or "unknown"
    machine = platform.machine() or "unknown"

    # processor is often empty on Linux; fall back to machine.
    processor = platform.processor() or machine or "unknown"

    # hardware_platform is not directly available in Python.
    # On Linux, this is typically the same as machine.
    hardware_platform = machine or "unknown"

    # operating_system: on Linux, it's "GNU/Linux"; on macOS, "Darwin".
    os_name = kernel_name
    if kernel_name == "Linux":
        os_name = "GNU/Linux"

    return {
        "kernel_name": kernel_name,
        "nodename": nodename,
        "kernel_release": kernel_release,
        "kernel_version": kernel_version,
        "machine": machine,
        "processor": processor,
        "hardware_platform": hardware_platform,
        "operating_system": os_name,
    }


def format_uname(
    info: dict[str, str],
    *,
    show_all: bool = False,
    show_kernel_name: bool = False,
    show_nodename: bool = False,
    show_kernel_release: bool = False,
    show_kernel_version: bool = False,
    show_machine: bool = False,
    show_processor: bool = False,
    show_hardware_platform: bool = False,
    show_operating_system: bool = False,
) -> str:
    """Format uname output based on which flags are set.

    If no flags are set, defaults to showing the kernel name (``-s``).
    If ``-a`` is set, all fields are shown in the canonical order.

    Args:
        info: Dictionary from ``get_system_info()``.
        show_all: If True, show all fields.
        show_kernel_name: Show kernel name (-s).
        show_nodename: Show nodename (-n).
        show_kernel_release: Show kernel release (-r).
        show_kernel_version: Show kernel version (-v).
        show_machine: Show machine (-m).
        show_processor: Show processor (-p).
        show_hardware_platform: Show hardware platform (-i).
        show_operating_system: Show operating system (-o).

    Returns:
        A single string with the requested fields, space-separated.
    """
    # The canonical order of fields in uname -a output.
    field_order = [
        ("kernel_name", show_kernel_name),
        ("nodename", show_nodename),
        ("kernel_release", show_kernel_release),
        ("kernel_version", show_kernel_version),
        ("machine", show_machine),
        ("processor", show_processor),
        ("hardware_platform", show_hardware_platform),
        ("operating_system", show_operating_system),
    ]

    if show_all:
        # Show all fields.
        parts = [info[name] for name, _ in field_order]
        return " ".join(parts)

    # Check if any specific flag was set.
    any_set = any(flag for _, flag in field_order)

    if not any_set:
        # Default: show kernel name only (same as -s).
        return info["kernel_name"]

    # Show only the requested fields, in canonical order.
    parts = [info[name] for name, flag in field_order if flag]
    return " ".join(parts)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then print system info."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"uname: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    info = get_system_info()
    output = format_uname(
        info,
        show_all=result.flags.get("all", False),
        show_kernel_name=result.flags.get("kernel_name", False),
        show_nodename=result.flags.get("nodename", False),
        show_kernel_release=result.flags.get("kernel_release", False),
        show_kernel_version=result.flags.get("kernel_version", False),
        show_machine=result.flags.get("machine", False),
        show_processor=result.flags.get("processor", False),
        show_hardware_platform=result.flags.get("hardware_platform", False),
        show_operating_system=result.flags.get("operating_system", False),
    )

    print(output)


if __name__ == "__main__":
    main()
