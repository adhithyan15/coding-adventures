# cmd.star -- Structured command builders for OS-aware BUILD rules.
#
# ============================================================================
# OVERVIEW
# ============================================================================
#
# This module provides functions for creating structured command dicts that
# the build tool renders into shell strings.  Instead of writing raw shell
# commands in BUILD files (which differ between Unix and Windows), BUILD
# rule authors use cmd() and its platform-specific variants.
#
# The build tool injects a _ctx dict into every Starlark scope.  This module
# reads _ctx["os"] at load time to determine the current platform.  Platform-
# specific functions return None when the OS doesn't match, and
# filter_commands() strips those None entries from the final command list.
#
# ============================================================================
# HOW IT WORKS
# ============================================================================
#
# 1. The build tool injects _ctx = {"os": "darwin", ...} before execution.
# 2. This module captures _ctx["os"] into _current_os at load time.
# 3. cmd() always returns a command dict.
# 4. cmd_windows() returns a dict only on Windows, None otherwise.
# 5. cmd_linux() returns a dict only on Linux, None otherwise.
# 6. cmd_macos() returns a dict only on macOS (darwin), None otherwise.
# 7. cmd_unix() returns a dict on any Unix (not Windows), None otherwise.
# 8. filter_commands() strips None entries from a list.
#
# The result is that BUILD rules compose commands like:
#
#   cmds = filter_commands([
#       cmd("cargo", ["build"]),
#       cmd_linux("cargo", ["tarpaulin"]),   # Linux-only coverage tool
#       cmd_unix("chmod", ["+x", "target"]), # Unix-only permissions
#   ])
#
# On Linux, all three commands appear.  On macOS, only the first and third.
# On Windows, only the first.
#
# ============================================================================
# _ctx DEPENDENCY
# ============================================================================
#
# This module REQUIRES _ctx to be injected by the build tool before loading.
# If _ctx is not present, the module will fail with a NameError.
#
# The _ctx dict must contain at least:
#   - "os": string -- one of "darwin", "linux", "windows", "freebsd", etc.
#
# See spec 15-os-aware-build-rules.md for the full _ctx v1 schema.
#
# ============================================================================
# EXTENDING TO NEW PLATFORMS
# ============================================================================
#
# To support a new platform (e.g., FreeBSD):
#
#   1. Add a new function:
#      def cmd_freebsd(program, args=[]):
#          if _current_os != "freebsd":
#              return None
#          return cmd(program, args)
#
#   2. That's it.  No build tool changes needed.  The build tool already
#      injects the correct "os" value from runtime.GOOS.
#

# ============================================================================
# PLATFORM DETECTION
# ============================================================================
#
# Capture _ctx["os"] once at module load time.  Functions in this module
# use this captured value rather than reading _ctx directly, because
# the VM's function call mechanism may use a separate scope where
# injected globals are not available.

_current_os = _ctx["os"]

# ============================================================================
# cmd() -- Universal command builder
# ============================================================================
#
# Creates a structured command dict that runs on all platforms.
#
# Args:
#   program: The executable to run (e.g., "cargo", "python", "npm").
#   args:    A list of arguments to pass to the program.
#            Default is an empty list.
#
# Returns:
#   A dict with keys:
#     - "type":    Always "cmd" (for future extensibility).
#     - "program": The executable name.
#     - "args":    The argument list.
#
# Example:
#   cmd("python", ["-m", "pytest"])
#   # => {"type": "cmd", "program": "python", "args": ["-m", "pytest"]}

def cmd(program, args=[]):
    return {"type": "cmd", "program": program, "args": args}

# ============================================================================
# Platform-specific command builders
# ============================================================================
#
# Each function checks _current_os (captured from _ctx["os"] at load time)
# and returns None if the current platform doesn't match.  The build tool's
# filter_commands() strips None entries before rendering.
#
# Why return None instead of raising an error?
#
#   Returning None lets BUILD rules compose platform-specific and universal
#   commands in a single list without if/else branching:
#
#     cmds = filter_commands([
#         cmd("cargo", ["build"]),           # Always
#         cmd_linux("cargo", ["tarpaulin"]), # Linux only
#     ])
#
#   On macOS, cmd_linux returns None, filter_commands strips it, and
#   the build tool only runs "cargo build".  Clean and declarative.

# cmd_windows -- Returns a command dict only on Windows.
#
# On any non-Windows platform, returns None.
# On Windows, delegates to cmd() to create the dict.

def cmd_windows(program, args=[]):
    if _current_os != "windows":
        return None
    return cmd(program, args)

# cmd_linux -- Returns a command dict only on Linux.
#
# On any non-Linux platform, returns None.
# On Linux, delegates to cmd() to create the dict.

def cmd_linux(program, args=[]):
    if _current_os != "linux":
        return None
    return cmd(program, args)

# cmd_macos -- Returns a command dict only on macOS (darwin).
#
# On any non-macOS platform, returns None.
# On macOS (os == "darwin"), delegates to cmd() to create the dict.

def cmd_macos(program, args=[]):
    if _current_os != "darwin":
        return None
    return cmd(program, args)

# cmd_unix -- Returns a command dict on any Unix platform (not Windows).
#
# Unix includes macOS, Linux, FreeBSD, OpenBSD, and any other non-Windows OS.
# On Windows, returns None.
#
# This is useful for commands that work on all Unix-like systems but not
# on Windows (e.g., chmod, sh scripts, 2>/dev/null redirects).

def cmd_unix(program, args=[]):
    if _current_os == "windows":
        return None
    return cmd(program, args)

# ============================================================================
# filter_commands() -- Remove None entries from a command list
# ============================================================================
#
# After composing platform-specific commands, some entries will be None
# (commands for other platforms).  filter_commands strips them out.
#
# Args:
#   cmds: A list that may contain command dicts and None values.
#
# Returns:
#   A new list with all None entries removed.
#
# Example:
#   cmds = [
#       cmd("cargo", ["build"]),
#       None,  # cmd_linux returned None on macOS
#       cmd("cargo", ["test"]),
#   ]
#   filter_commands(cmds)
#   # => [{"type": "cmd", ...}, {"type": "cmd", ...}]

def filter_commands(cmds):
    result = []
    for c in cmds:
        if c != None:
            result.append(c)
    return result
