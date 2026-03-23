#!/usr/bin/env ruby
# frozen_string_literal: true

# xargs_tool.rb -- Build and execute command lines from standard input
# ======================================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `xargs` utility. It reads items
# from standard input (delimited by whitespace or newlines by default),
# and executes a specified command with those items as arguments.
#
# === How xargs Works ===
#
#     $ echo "file1 file2 file3" | xargs rm       # rm file1 file2 file3
#     $ find . -name "*.tmp" | xargs rm            # delete all .tmp files
#     $ find . -name "*.tmp" -print0 | xargs -0 rm # handle filenames with spaces
#     $ echo "a b c" | xargs -n 1 echo             # echo a; echo b; echo c
#     $ echo "a b" | xargs -I {} cp {} /backup/    # replace {} with each item
#
# === Input Parsing ===
#
# By default, xargs splits input on whitespace (spaces, tabs, newlines)
# and handles quoting:
#   - Single quotes: 'hello world' is one argument
#   - Double quotes: "hello world" is one argument
#   - Backslash: hello\ world is one argument
#
# With -0 (null): items are separated by null bytes, no quote processing.
# With -d DELIM: items are separated by DELIM, no quote processing.
#
# === Batching ===
#
# Without -n or -L, xargs appends as many items as possible to a single
# command invocation (up to the system limit).
# With -n MAX, at most MAX items are used per invocation.
# With -I STR, one item at a time, replacing STR in the command template.

require "open3"
require "shellwords"
require "coding_adventures_cli_builder"

XARGS_SPEC_FILE = File.join(File.dirname(__FILE__), "xargs.json")

# ---------------------------------------------------------------------------
# Business Logic: xargs_parse_items
# ---------------------------------------------------------------------------
# Parse input text into individual items according to the delimiter mode.
#
# Modes:
#   :null       - Split on null bytes (no quote processing)
#   :delimiter  - Split on a specific character (no quote processing)
#   :default    - Split on whitespace with quote processing
#
# Parameters:
#   input    - The raw input string from stdin
#   opts     - Hash with :null (boolean), :delimiter (string or nil)
#
# Returns: Array of item strings.

def xargs_parse_items(input, opts = {})
  if opts[:null]
    # -0 mode: split on null bytes, ignore trailing empty items
    input.split("\0").reject(&:empty?)
  elsif opts[:delimiter]
    # -d mode: split on a specific delimiter
    delim = opts[:delimiter]
    # Handle escape sequences in delimiter
    delim = "\n" if delim == "\\n"
    delim = "\t" if delim == "\\t"
    input.split(delim).reject(&:empty?)
  else
    # Default mode: split on whitespace with quote handling.
    # We use Shellwords.shellsplit which handles single quotes,
    # double quotes, and backslash escaping -- exactly like xargs.
    begin
      Shellwords.shellsplit(input)
    rescue ArgumentError
      # If there's a quoting error, fall back to simple whitespace split
      input.split
    end
  end
end

# ---------------------------------------------------------------------------
# Business Logic: xargs_build_batches
# ---------------------------------------------------------------------------
# Group items into batches according to -n (max_args) and -I (replace).
#
# Parameters:
#   items    - Array of input items
#   command  - The command template (array of strings)
#   opts     - Hash with :max_args, :replace, :max_lines
#
# Returns: Array of command arrays, each ready to be executed.

def xargs_build_batches(items, command, opts = {})
  # Default command is /bin/echo if none specified
  command = ["/bin/echo"] if command.nil? || command.empty?

  if opts[:replace]
    # -I mode: one item per invocation, replace placeholder in command
    placeholder = opts[:replace]
    items.map do |item|
      command.map { |arg| arg.gsub(placeholder, item) }
    end
  elsif opts[:max_args]
    # -n mode: batch items into groups of max_args
    items.each_slice(opts[:max_args]).map do |batch|
      command + batch
    end
  else
    # Default: all items in one command
    [command + items]
  end
end

# ---------------------------------------------------------------------------
# Business Logic: xargs_execute
# ---------------------------------------------------------------------------
# Execute a series of command batches and return the overall exit code.
#
# Parameters:
#   batches  - Array of command arrays from xargs_build_batches
#   opts     - Hash with :verbose (boolean), :no_run_if_empty (boolean)
#   io_err   - IO for stderr output (default: $stderr)
#
# Returns: Integer exit code (0 = all succeeded, 123 = some failed, 127 = not found)

def xargs_execute(batches, opts = {}, io_err: $stderr)
  return 0 if batches.empty?

  overall_exit = 0

  batches.each do |cmd|
    # Print command to stderr if verbose (-t)
    if opts[:verbose]
      io_err.puts cmd.shelljoin
    end

    # Execute the command
    success = system(*cmd)

    if success.nil?
      # Command not found
      io_err.puts "xargs: #{cmd[0]}: No such file or directory"
      return 127
    elsif !success
      overall_exit = 123
    end
  end

  overall_exit
end

# ---------------------------------------------------------------------------
# Business Logic: xargs_run
# ---------------------------------------------------------------------------
# High-level function: parse input, build batches, execute.
#
# Parameters:
#   input    - Raw input string (from stdin or file)
#   command  - Command template (array of strings, or nil for echo)
#   opts     - Options hash combining all xargs options
#   io_err   - IO for stderr (default $stderr)
#
# Returns: Integer exit code

def xargs_run(input, command, opts = {}, io_err: $stderr)
  items = xargs_parse_items(input, opts)

  # -r: don't run if input is empty
  if opts[:no_run_if_empty] && items.empty?
    return 0
  end

  # If no items and no -r, xargs still runs the command once with no args
  # (unless -I is set, in which case no invocations happen)
  if items.empty? && opts[:replace]
    return 0
  end

  batches = xargs_build_batches(items, command, opts)
  xargs_execute(batches, opts, io_err: io_err)
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def xargs_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(XARGS_SPEC_FILE, ["xargs"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "xargs: #{err.message}" }
    exit 1
  end

  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    command_args = result.arguments.fetch("command", [])
    command_args = [command_args] if command_args.is_a?(String)

    opts = {
      null: result.flags["null"] || false,
      delimiter: result.flags["delimiter"],
      max_args: result.flags["max_args"],
      replace: result.flags["replace"],
      verbose: result.flags["verbose"] || false,
      no_run_if_empty: result.flags["no_run_if_empty"] || false,
    }

    # Read input from file or stdin
    input = if result.flags["arg_file"]
              File.read(result.flags["arg_file"])
            else
              $stdin.read
            end

    exit_code = xargs_run(input, command_args, opts)
    exit exit_code
  end
end

xargs_main if __FILE__ == $PROGRAM_NAME
