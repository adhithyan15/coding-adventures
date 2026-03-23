#!/usr/bin/env ruby
# frozen_string_literal: true

# env_tool.rb -- Run a program in a modified environment
# ========================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `env` utility. It can:
# 1. Print the current environment (when called with no arguments)
# 2. Run a command with a modified environment
#
# === How env Works ===
#
#     $ env                            # print all environment variables
#     $ env FOO=bar command            # run command with FOO=bar added
#     $ env -i command                 # run command with empty environment
#     $ env -u HOME command            # run command without HOME variable
#     $ env -0                         # print env with NUL terminators
#
# === Environment Modification ===
#
# The arguments to env are processed left to right. Any argument that
# contains '=' is treated as a variable assignment (NAME=VALUE). The
# first argument that does NOT contain '=' is treated as the command
# to execute, and all subsequent arguments are passed to that command.
#
# Special modes:
#   -i  Start with an empty environment (ignore inherited environment)
#   -u  Unset a specific variable before running the command
#   -C  Change directory before running the command
#   -0  Use NUL instead of newline as output separator (for printing)

require "coding_adventures_cli_builder"

ENV_SPEC_FILE = File.join(File.dirname(__FILE__), "env.json")

# ---------------------------------------------------------------------------
# Business Logic: env_parse_assignments
# ---------------------------------------------------------------------------
# Separate the positional arguments into variable assignments and the
# command to run. Arguments of the form NAME=VALUE are assignments;
# the first non-assignment argument starts the command.
#
# Parameters:
#   args - Array of positional argument strings
#
# Returns: [assignments_hash, command_array]
#   assignments_hash: {"VAR" => "value", ...}
#   command_array: ["cmd", "arg1", "arg2", ...] or [] if no command

def env_parse_assignments(args)
  assignments = {}
  command = []
  found_command = false

  args.each do |arg|
    if found_command
      command << arg
    elsif arg.include?("=")
      key, value = arg.split("=", 2)
      assignments[key] = value
    else
      found_command = true
      command << arg
    end
  end

  [assignments, command]
end

# ---------------------------------------------------------------------------
# Business Logic: env_build_environment
# ---------------------------------------------------------------------------
# Build the environment hash for the child process.
#
# Parameters:
#   base_env     - The starting environment (Hash). Use ENV.to_h for current
#                  environment, or {} for -i (ignore) mode.
#   assignments  - Hash of NAME=VALUE pairs to set
#   unset_vars   - Array of variable names to unset
#
# Returns: A new Hash representing the desired environment.

def env_build_environment(base_env, assignments, unset_vars = [])
  env = base_env.dup

  # Remove unset variables
  unset_vars.each { |var| env.delete(var) }

  # Apply assignments (these override unsets)
  assignments.each { |key, value| env[key] = value }

  env
end

# ---------------------------------------------------------------------------
# Business Logic: env_print_environment
# ---------------------------------------------------------------------------
# Format the environment for printing. Each variable is printed as
# NAME=VALUE, separated by newlines (or NUL if null_terminated is true).
#
# Parameters:
#   env_hash        - Hash of environment variables
#   null_terminated - Boolean, if true use NUL instead of newline
#
# Returns: The formatted string.

def env_print_environment(env_hash, null_terminated: false)
  terminator = null_terminated ? "\0" : "\n"
  lines = env_hash.map { |key, value| "#{key}=#{value}" }
  return "" if lines.empty? && null_terminated
  return "\n" if lines.empty?
  lines.map { |l| l + terminator }.join
end

# ---------------------------------------------------------------------------
# Business Logic: env_execute
# ---------------------------------------------------------------------------
# Execute a command in the given environment, optionally changing directory.
#
# Parameters:
#   env_hash  - Hash of environment variables for the child process
#   command   - Array of command + arguments
#   chdir     - Directory to change to (or nil)
#
# Returns: exit code (Integer), or nil if command not found

def env_execute(env_hash, command, chdir: nil)
  opts = {}
  opts[:chdir] = chdir if chdir

  # Use exec-style call: replace the environment entirely.
  # The [env, *cmd] form of system() sets the child's environment.
  pid = spawn(env_hash, *command, **opts)
  _, status = Process.wait2(pid)
  status.exitstatus || 125
rescue Errno::ENOENT
  $stderr.puts "env: '#{command.first}': No such file or directory"
  127
rescue Errno::EACCES
  $stderr.puts "env: '#{command.first}': Permission denied"
  126
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def env_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(ENV_SPEC_FILE, ["env"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "env: #{err.message}" }
    exit 125
  end

  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    raw_args = result.arguments.fetch("assignments_and_command", [])
    raw_args = [raw_args] if raw_args.is_a?(String)

    ignore_env = result.flags["ignore_environment"] || false
    null_terminated = result.flags["null"] || false
    unset_vars = result.flags["unset"] || []
    unset_vars = [unset_vars] if unset_vars.is_a?(String)
    chdir = result.flags["chdir"]

    # Parse assignments and command from positional args
    assignments, command = env_parse_assignments(raw_args)

    # Build the environment
    base_env = ignore_env ? {} : ENV.to_h
    env_hash = env_build_environment(base_env, assignments, unset_vars)

    if command.empty?
      # No command: print the environment
      print env_print_environment(env_hash, null_terminated: null_terminated)
      exit 0
    else
      # Execute the command in the modified environment
      code = env_execute(env_hash, command, chdir: chdir)
      exit code
    end
  end
end

env_main if __FILE__ == $PROGRAM_NAME
