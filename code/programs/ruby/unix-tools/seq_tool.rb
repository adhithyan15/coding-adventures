#!/usr/bin/env ruby
# frozen_string_literal: true

# seq_tool.rb -- Print a sequence of numbers
# ============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `seq` utility. It prints a
# sequence of numbers from FIRST to LAST, separated by a string
# (default: newline).
#
# === Argument Forms ===
#
# seq accepts 1, 2, or 3 positional arguments:
#
#   seq LAST              -> prints 1 to LAST (increment 1)
#   seq FIRST LAST        -> prints FIRST to LAST (increment 1 or -1)
#   seq FIRST INCR LAST   -> prints FIRST to LAST (increment INCR)
#
# The numbers can be integers or floating-point:
#
#     $ seq 5
#     1 2 3 4 5
#
#     $ seq 2 0.5 4
#     2.0 2.5 3.0 3.5 4.0
#
# === Equal Width (-w) ===
#
# The -w flag pads all numbers with leading zeros so they are the
# same width:
#
#     $ seq -w 8 10
#     08
#     09
#     10
#
# === Custom Separator (-s) ===
#
# By default, numbers are separated by newlines. The -s flag lets
# you choose a different separator:
#
#     $ seq -s ", " 3
#     1, 2, 3
#
# The output always ends with a newline, regardless of the separator.
#
# === Format (-f) ===
#
# The -f flag lets you specify a printf-style format string:
#
#     $ seq -f "Item %03g" 3
#     Item 001
#     Item 002
#     Item 003

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SEQ_SPEC_FILE = File.join(File.dirname(__FILE__), "seq.json")

# ---------------------------------------------------------------------------
# Business Logic: parse_seq_args
# ---------------------------------------------------------------------------
# Parse the 1-3 positional arguments into first, increment, last.
#
# seq LAST           -> first=1, incr=1, last=LAST
# seq FIRST LAST     -> first=FIRST, incr=1 (or -1 if FIRST > LAST), last=LAST
# seq FIRST INCR LAST -> first=FIRST, incr=INCR, last=LAST
#
# Returns: [Float, Float, Float] -- first, increment, last

def parse_seq_args(numbers)
  case numbers.length
  when 1
    [1.0, 1.0, Float(numbers[0])]
  when 2
    first = Float(numbers[0])
    last = Float(numbers[1])
    incr = first <= last ? 1.0 : -1.0
    [first, incr, last]
  when 3
    [Float(numbers[0]), Float(numbers[1]), Float(numbers[2])]
  else
    raise ArgumentError, "seq: expected 1-3 arguments, got #{numbers.length}"
  end
end

# ---------------------------------------------------------------------------
# Business Logic: format_number
# ---------------------------------------------------------------------------
# Format a number for output. If the number is an integer value (no
# fractional part), format it as an integer. Otherwise, format as float.
# This matches GNU seq behavior: `seq 3` prints "1\n2\n3" not "1.0\n2.0\n3.0".
#
# Parameters:
#   num        - The number to format
#   fmt        - Optional printf format string (overrides auto-detection)
#   pad_width  - Width for zero-padding (0 for no padding)
#
# Returns: The formatted number string.

def format_seq_number(num, fmt, pad_width)
  if fmt
    # User-specified format string.
    format(fmt, num)
  elsif num == num.floor && num.abs < 1e15
    # Integer value: format without decimal point.
    str = num.to_i.to_s
    if pad_width > 0
      # Zero-pad to the required width. Negative numbers get special
      # treatment: the minus sign counts toward the width.
      if num < 0
        "-" + str[1..].rjust(pad_width - 1, "0")
      else
        str.rjust(pad_width, "0")
      end
    else
      str
    end
  else
    # Floating-point value: use %g for clean output.
    str = format("%g", num)
    if pad_width > 0
      if num < 0
        "-" + str[1..].rjust(pad_width - 1, "0")
      else
        str.rjust(pad_width, "0")
      end
    else
      str
    end
  end
end

# ---------------------------------------------------------------------------
# Business Logic: compute_pad_width
# ---------------------------------------------------------------------------
# Compute the width needed for zero-padding with -w.
#
# The width is determined by the widest number that will be printed,
# which is either the first or last value.

def compute_pad_width(first, last)
  # Format first and last as they would appear (without padding) and
  # take the longer one's width.
  w1 = if first == first.floor && first.abs < 1e15
         first.to_i.to_s.length
       else
         format("%g", first).length
       end
  w2 = if last == last.floor && last.abs < 1e15
         last.to_i.to_s.length
       else
         format("%g", last).length
       end
  [w1, w2].max
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def seq_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(SEQ_SPEC_FILE, ["seq"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "seq: #{err.message}" }
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
    numbers = result.arguments.fetch("numbers", [])
    separator = result.flags["separator"] || "\n"
    equal_width = result.flags["equal_width"]
    fmt = result.flags["format"]

    begin
      first, incr, last = parse_seq_args(numbers)
    rescue ArgumentError => e
      warn e.message
      exit 1
    end

    # Cannot iterate with zero increment.
    if incr == 0.0
      warn "seq: zero increment"
      exit 1
    end

    # Compute zero-padding width if -w is set.
    pad_width = equal_width ? compute_pad_width(first, last) : 0

    # Generate the sequence. We collect into an array first so we can
    # join with the separator, then print with a trailing newline.
    #
    # The loop condition depends on the direction of the increment:
    #   - Positive increment: keep going while current <= last
    #   - Negative increment: keep going while current >= last
    #
    # We add a small epsilon tolerance to handle floating-point
    # rounding errors (e.g., 0.1 + 0.1 + 0.1 might be 0.30000000000000004).
    values = []
    current = first
    epsilon = (incr.abs * 1e-10)

    if incr > 0
      while current <= last + epsilon
        values << format_seq_number(current, fmt, pad_width)
        current += incr
      end
    else
      while current >= last - epsilon
        values << format_seq_number(current, fmt, pad_width)
        current += incr
      end
    end

    print values.join(separator)
    puts
  end
end

# Only run main when this file is executed directly.
seq_main if __FILE__ == $PROGRAM_NAME
