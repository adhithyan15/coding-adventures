#!/usr/bin/env ruby
# frozen_string_literal: true

# uname_tool.rb -- Print system information
# ===========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `uname` utility. It prints
# information about the operating system and hardware. With no flags,
# it prints the kernel name (same as -s).
#
# === Information Fields ===
#
# uname can display several pieces of system information:
#
#   -s   Kernel name (e.g., "Darwin", "Linux")
#   -n   Network node hostname (e.g., "my-macbook")
#   -r   Kernel release (e.g., "23.4.0", "6.5.0-44-generic")
#   -v   Kernel version (build info string)
#   -m   Machine hardware name (e.g., "arm64", "x86_64")
#   -p   Processor type (e.g., "arm", "x86_64")
#   -i   Hardware platform (non-portable)
#   -o   Operating system (e.g., "Darwin", "GNU/Linux")
#   -a   Print all of the above in order: s n r v m p i o
#
# === How We Get This Information ===
#
# Ruby provides several ways to access system information:
#
# 1. `RUBY_PLATFORM` — Contains the CPU-OS pair (e.g., "arm64-darwin23")
# 2. `Socket.gethostname` — Returns the hostname
# 3. `Etc.uname` — Ruby 2.7+ hash with :sysname, :nodename, :release,
#    :version, :machine keys (maps to POSIX uname(2) system call)
# 4. `RbConfig::CONFIG` — Ruby's build configuration, includes host info
#
# We prefer `Etc.uname` when available because it maps directly to the
# POSIX uname(2) system call, giving us the most accurate information.

require "etc"
require "socket"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

UNAME_SPEC_FILE = File.join(File.dirname(__FILE__), "uname.json")

# ---------------------------------------------------------------------------
# Business Logic: get_system_info
# ---------------------------------------------------------------------------
# Gather all system information into a hash.
#
# We use Etc.uname (Ruby 2.7+) as the primary source, falling back
# to RUBY_PLATFORM and Socket for older Rubies.
#
# Returns: A hash with keys :kernel_name, :nodename, :kernel_release,
#          :kernel_version, :machine, :processor, :hardware_platform,
#          :operating_system.

def get_system_info
  info = {}

  if Etc.respond_to?(:uname)
    # Ruby 2.7+ provides Etc.uname which wraps the POSIX uname(2) call
    uname_data = Etc.uname
    info[:kernel_name]     = uname_data[:sysname] || "unknown"
    info[:nodename]        = uname_data[:nodename] || "unknown"
    info[:kernel_release]  = uname_data[:release] || "unknown"
    info[:kernel_version]  = uname_data[:version] || "unknown"
    info[:machine]         = uname_data[:machine] || "unknown"
  else
    # Fallback for older Ruby versions
    info[:kernel_name]     = detect_kernel_name
    info[:nodename]        = Socket.gethostname
    info[:kernel_release]  = "unknown"
    info[:kernel_version]  = "unknown"
    info[:machine]         = detect_machine
  end

  # Processor type: often same as machine, or detected from platform
  info[:processor] = info[:machine]

  # Hardware platform: non-portable, often same as machine
  info[:hardware_platform] = info[:machine]

  # Operating system: derived from kernel name
  info[:operating_system] = detect_operating_system(info[:kernel_name])

  info
end

# ---------------------------------------------------------------------------
# Business Logic: detect_kernel_name
# ---------------------------------------------------------------------------
# Detect the kernel name from RUBY_PLATFORM.
#
# RUBY_PLATFORM contains strings like "arm64-darwin23", "x86_64-linux",
# "x86_64-mingw32". We extract the OS part.

def detect_kernel_name
  case RUBY_PLATFORM
  when /darwin/i then "Darwin"
  when /linux/i then "Linux"
  when /mingw|mswin/i then "Windows_NT"
  when /freebsd/i then "FreeBSD"
  else "unknown"
  end
end

# ---------------------------------------------------------------------------
# Business Logic: detect_machine
# ---------------------------------------------------------------------------
# Detect the machine hardware name from RUBY_PLATFORM.

def detect_machine
  case RUBY_PLATFORM
  when /arm64|aarch64/i then "arm64"
  when /x86_64|x64/i then "x86_64"
  when /i[3-6]86/i then "i686"
  else "unknown"
  end
end

# ---------------------------------------------------------------------------
# Business Logic: detect_operating_system
# ---------------------------------------------------------------------------
# Map kernel names to operating system names.
#
# On Linux, the OS is typically "GNU/Linux". On macOS, it's "Darwin".
# This matches GNU coreutils behavior.

def detect_operating_system(kernel_name)
  case kernel_name
  when "Linux" then "GNU/Linux"
  when "Darwin" then "Darwin"
  when "FreeBSD" then "FreeBSD"
  else kernel_name
  end
end

# ---------------------------------------------------------------------------
# Business Logic: format_uname
# ---------------------------------------------------------------------------
# Format the system information based on the requested flags.
#
# When -a is specified, all fields are printed in order:
#   kernel_name nodename kernel_release kernel_version machine processor
#   hardware_platform operating_system
#
# When no flags are specified, only the kernel name is printed (same as -s).
#
# Parameters:
#   info  - Hash from get_system_info
#   flags - Hash of flag values from CLI Builder
#
# Returns: A formatted string.

def format_uname(info, flags)
  # If -a is set, enable all fields
  show_all = flags["all"]

  parts = []

  if show_all || flags["kernel_name"] || no_uname_flags?(flags)
    parts << info[:kernel_name]
  end
  parts << info[:nodename] if show_all || flags["nodename"]
  parts << info[:kernel_release] if show_all || flags["kernel_release"]
  parts << info[:kernel_version] if show_all || flags["kernel_version"]
  parts << info[:machine] if show_all || flags["machine"]
  parts << info[:processor] if show_all || flags["processor"]
  parts << info[:hardware_platform] if show_all || flags["hardware_platform"]
  parts << info[:operating_system] if show_all || flags["operating_system"]

  parts.join(" ")
end

# ---------------------------------------------------------------------------
# Helper: Check if no uname-specific flags are set
# ---------------------------------------------------------------------------
# When no flags are given, uname defaults to printing just -s.

def no_uname_flags?(flags)
  !flags["all"] &&
    !flags["kernel_name"] &&
    !flags["nodename"] &&
    !flags["kernel_release"] &&
    !flags["kernel_version"] &&
    !flags["machine"] &&
    !flags["processor"] &&
    !flags["hardware_platform"] &&
    !flags["operating_system"]
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def uname_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(UNAME_SPEC_FILE, ["uname"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "uname: #{err.message}" }
    exit 1
  end

  # --- Step 2: Dispatch on result type -------------------------------------
  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    # --- Step 3: Business logic --------------------------------------------
    info = get_system_info
    puts format_uname(info, result.flags)
  end
end

# Only run main when this file is executed directly.
uname_main if __FILE__ == $PROGRAM_NAME
