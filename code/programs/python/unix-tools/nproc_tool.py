"""nproc — print the number of processing units available.

=== What This Program Does ===

This is a reimplementation of the GNU ``nproc`` utility. It prints the
number of processing units (CPU cores) available to the current process.

=== Why Does This Exist? ===

Many programs can run faster by doing work in parallel. But how many
parallel workers should you start? Too few wastes potential, too many
causes thrashing. ``nproc`` gives you the answer::

    make -j$(nproc)      # Use all available cores for compilation
    parallel -j$(nproc)  # Run GNU parallel with optimal parallelism

=== Available vs Installed Processors ===

On modern systems, the number of *available* processors may be less than
the total *installed*:

- **Available**: CPUs the current process is allowed to use. This can be
  restricted by ``taskset``, ``cgroups``, Docker ``--cpus``, or
  ``sched_setaffinity()``.
- **Installed**: The total number of physical/logical CPUs in the machine.

By default, ``nproc`` reports available processors. With ``--all``, it
reports installed processors.

=== The --ignore Flag ===

``--ignore=N`` subtracts N from the CPU count, with a minimum of 1.
This is useful when you want to leave some cores free::

    make -j$(nproc --ignore=2)   # Leave 2 cores for other work

=== How It Works ===

On Linux, ``os.sched_getaffinity(0)`` returns the set of CPUs the
current process can use. On macOS and other platforms, this function
doesn't exist, so we fall back to ``os.cpu_count()``.

=== CLI Builder Integration ===

The JSON spec ``nproc.json`` defines two flags: ``--all`` (boolean) and
``--ignore`` (integer). CLI Builder handles all the parsing.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

SPEC_FILE = str(Path(__file__).parent / "nproc.json")


# ---------------------------------------------------------------------------
# Business logic
# ---------------------------------------------------------------------------


def get_available_cpus() -> int:
    """Return the number of CPUs available to the current process.

    On Linux, we use ``os.sched_getaffinity(0)`` which respects CPU
    affinity masks (set by ``taskset``, cgroups, Docker ``--cpus``, etc.).
    The ``0`` means "the current process."

    On macOS and other platforms where ``sched_getaffinity`` is not
    available, we fall back to ``os.cpu_count()``.

    If even ``os.cpu_count()`` returns ``None`` (which would be very
    unusual), we return 1 as a safe default.

    Returns:
        The number of available CPUs, always >= 1.
    """
    # Try the Linux-specific affinity API first.
    if hasattr(os, "sched_getaffinity"):
        return len(os.sched_getaffinity(0))

    # Fall back to os.cpu_count() on macOS and other platforms.
    count = os.cpu_count()
    return count if count is not None else 1


def get_installed_cpus() -> int:
    """Return the total number of installed CPUs.

    This ignores any affinity restrictions and reports the full number
    of logical CPUs in the system. On a machine with 8 cores, this
    always returns 8 even if the process is restricted to 2 cores.

    We use ``os.cpu_count()`` which queries the OS for the total number
    of logical processors. If it returns ``None`` (very rare — only on
    exotic platforms), we default to 1.

    Returns:
        The number of installed CPUs, always >= 1.
    """
    count = os.cpu_count()
    return count if count is not None else 1


def apply_ignore(cpu_count: int, ignore: int) -> int:
    """Subtract *ignore* from *cpu_count*, but never go below 1.

    This implements the ``--ignore=N`` flag. The idea is that you might
    want to leave some cores free for other work::

        >>> apply_ignore(8, 2)
        6
        >>> apply_ignore(4, 0)
        4
        >>> apply_ignore(2, 5)
        1

    The minimum is always 1 because you need at least one processing
    unit to do any work at all.

    Args:
        cpu_count: The base number of CPUs.
        ignore: How many to subtract.

    Returns:
        max(1, cpu_count - ignore)
    """
    return max(1, cpu_count - ignore)


def main() -> None:
    """Entry point: parse args via CLI Builder, then print CPU count."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"nproc: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    assert isinstance(result, ParseResult)

    # Determine whether to count all installed CPUs or just available ones.
    use_all = result.flags.get("all", False)

    count = get_installed_cpus() if use_all else get_available_cpus()

    # Apply the --ignore flag if present.
    ignore = result.flags.get("ignore", 0)
    if ignore:
        count = apply_ignore(count, ignore)

    print(count)


if __name__ == "__main__":
    main()
