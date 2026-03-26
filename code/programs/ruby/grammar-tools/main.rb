#!/usr/bin/env ruby
# frozen_string_literal: true

# grammar-tools — CLI for validating .tokens and .grammar files.
#
# This program wraps the CodingAdventures::GrammarTools library behind a
# CodingAdventures::CliBuilder-powered interface. It is the Ruby counterpart
# of the Python, Elixir, Go, Rust, TypeScript, and Lua implementations.
# All produce identical output so CI scripts can use any implementation.
#
# Usage
# -----
#
#   ruby main.rb validate <file.tokens> <file.grammar>
#   ruby main.rb validate-tokens <file.tokens>
#   ruby main.rb validate-grammar <file.grammar>
#   ruby main.rb --help
#
# Exit codes
# ----------
#
#   0 — all checks passed
#   1 — one or more validation errors found
#   2 — usage error (wrong number of arguments, unknown command)

require "json"
require "pathname"

# ---------------------------------------------------------------------------
# Locate the repo root and set up load paths.
#
# Walk up from this file until we find code/specs/grammar-tools.json.
# ---------------------------------------------------------------------------

ROOT = begin
  dir = Pathname.new(__FILE__).expand_path.dirname
  20.times do
    break dir if (dir / "code" / "specs" / "grammar-tools.json").exist?
    parent = dir.parent
    break dir if parent == dir
    dir = parent
  end
  dir
end

$LOAD_PATH.unshift(ROOT / "code/packages/ruby/grammar_tools/lib")
$LOAD_PATH.unshift(ROOT / "code/packages/ruby/directed_graph/lib")
$LOAD_PATH.unshift(ROOT / "code/packages/ruby/state_machine/lib")
$LOAD_PATH.unshift(ROOT / "code/packages/ruby/cli_builder/lib")

require "coding_adventures_grammar_tools"
require "coding_adventures_cli_builder"

GT  = CodingAdventures::GrammarTools
CLI = CodingAdventures::CliBuilder

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

# Count issues that are actual errors, not informational warnings.
#
# Issues beginning with "Warning:" are shown but do not cause non-zero exit.
def count_errors(issues)
  issues.count { |i| !i.start_with?("Warning:") }
end

# Print issues with two-space indentation.
def print_issues(issues)
  issues.each { |i| puts "  #{i}" }
end

def print_usage
  $stderr.puts "Usage: grammar-tools <command> [args...]"
  $stderr.puts
  $stderr.puts "Commands:"
  $stderr.puts "  validate <file.tokens> <file.grammar>  Validate a token/grammar pair"
  $stderr.puts "  validate-tokens <file.tokens>           Validate just a .tokens file"
  $stderr.puts "  validate-grammar <file.grammar>         Validate just a .grammar file"
  $stderr.puts
  $stderr.puts "Run 'grammar-tools --help' for full help text."
end

# ---------------------------------------------------------------------------
# validate — cross-validate a .tokens/.grammar pair
# ---------------------------------------------------------------------------

def validate_command(tokens_path, grammar_path)
  total_errors = 0

  # Step 1: parse and validate .tokens
  unless File.exist?(tokens_path)
    $stderr.puts "Error: File not found: #{tokens_path}"
    return 1
  end

  print "Validating #{File.basename(tokens_path)} ... "
  begin
    token_grammar = GT.parse_token_grammar(File.read(tokens_path))
  rescue GT::TokenGrammarError => e
    puts "PARSE ERROR"
    puts "  #{e}"
    return 1
  end

  token_issues = GT.validate_token_grammar(token_grammar)
  token_errors = count_errors(token_issues)
  n_tokens = token_grammar.definitions.length
  n_skip   = token_grammar.skip_definitions.length
  n_error  = token_grammar.respond_to?(:error_definitions) ? token_grammar.error_definitions.length : 0

  if token_errors > 0
    puts "#{token_errors} error(s)"
    print_issues(token_issues)
    total_errors += token_errors
  else
    parts = ["#{n_tokens} tokens"]
    parts << "#{n_skip} skip"  if n_skip > 0
    parts << "#{n_error} error" if n_error > 0
    puts "OK (#{parts.join(", ")})"
  end

  # Step 2: parse and validate .grammar
  unless File.exist?(grammar_path)
    $stderr.puts "Error: File not found: #{grammar_path}"
    return 1
  end

  print "Validating #{File.basename(grammar_path)} ... "
  begin
    parser_grammar = GT.parse_parser_grammar(File.read(grammar_path))
  rescue GT::ParserGrammarError => e
    puts "PARSE ERROR"
    puts "  #{e}"
    return 1
  end

  grammar_issues = GT.validate_parser_grammar(
    parser_grammar,
    token_names: token_grammar.token_names
  )
  grammar_errors = count_errors(grammar_issues)
  n_rules = parser_grammar.rules.length

  if grammar_errors > 0
    puts "#{grammar_errors} error(s)"
    print_issues(grammar_issues)
    total_errors += grammar_errors
  else
    puts "OK (#{n_rules} rules)"
  end

  # Step 3: cross-validate
  print "Cross-validating ... "
  cross_issues   = GT.cross_validate(token_grammar, parser_grammar)
  cross_errors   = count_errors(cross_issues)
  cross_warnings = cross_issues.length - cross_errors

  if cross_errors > 0
    puts "#{cross_errors} error(s)"
    print_issues(cross_issues)
    total_errors += cross_errors
  elsif cross_warnings > 0
    puts "OK (#{cross_warnings} warning(s))"
    print_issues(cross_issues)
  else
    puts "OK"
  end

  puts
  if total_errors > 0
    puts "Found #{total_errors} error(s). Fix them and try again."
    1
  else
    puts "All checks passed."
    0
  end
end

# ---------------------------------------------------------------------------
# validate-tokens — validate just a .tokens file
# ---------------------------------------------------------------------------

def validate_tokens_only(tokens_path)
  unless File.exist?(tokens_path)
    $stderr.puts "Error: File not found: #{tokens_path}"
    return 1
  end

  print "Validating #{File.basename(tokens_path)} ... "
  begin
    token_grammar = GT.parse_token_grammar(File.read(tokens_path))
  rescue GT::TokenGrammarError => e
    puts "PARSE ERROR"
    puts "  #{e}"
    return 1
  end

  issues  = GT.validate_token_grammar(token_grammar)
  errors  = count_errors(issues)
  n_tokens = token_grammar.definitions.length
  n_skip   = token_grammar.skip_definitions.length
  n_error  = token_grammar.respond_to?(:error_definitions) ? token_grammar.error_definitions.length : 0

  if errors > 0
    puts "#{errors} error(s)"
    print_issues(issues)
    puts
    puts "Found #{errors} error(s). Fix them and try again."
    1
  else
    parts = ["#{n_tokens} tokens"]
    parts << "#{n_skip} skip"  if n_skip > 0
    parts << "#{n_error} error" if n_error > 0
    puts "OK (#{parts.join(", ")})"
    puts
    puts "All checks passed."
    0
  end
end

# ---------------------------------------------------------------------------
# validate-grammar — validate just a .grammar file
# ---------------------------------------------------------------------------

def validate_grammar_only(grammar_path)
  unless File.exist?(grammar_path)
    $stderr.puts "Error: File not found: #{grammar_path}"
    return 1
  end

  print "Validating #{File.basename(grammar_path)} ... "
  begin
    parser_grammar = GT.parse_parser_grammar(File.read(grammar_path))
  rescue GT::ParserGrammarError => e
    puts "PARSE ERROR"
    puts "  #{e}"
    return 1
  end

  issues  = GT.validate_parser_grammar(parser_grammar)
  errors  = count_errors(issues)
  n_rules = parser_grammar.rules.length

  if errors > 0
    puts "#{errors} error(s)"
    print_issues(issues)
    puts
    puts "Found #{errors} error(s). Fix them and try again."
    1
  else
    puts "OK (#{n_rules} rules)"
    puts
    puts "All checks passed."
    0
  end
end

# ---------------------------------------------------------------------------
# dispatch
# ---------------------------------------------------------------------------

def dispatch(command, files)
  case command
  when "validate"
    if files.length != 2
      $stderr.puts "Error: 'validate' requires two arguments: <tokens> <grammar>"
      $stderr.puts
      print_usage
      return 2
    end
    validate_command(files[0], files[1])

  when "validate-tokens"
    if files.length != 1
      $stderr.puts "Error: 'validate-tokens' requires one argument: <tokens>"
      $stderr.puts
      print_usage
      return 2
    end
    validate_tokens_only(files[0])

  when "validate-grammar"
    if files.length != 1
      $stderr.puts "Error: 'validate-grammar' requires one argument: <grammar>"
      $stderr.puts
      print_usage
      return 2
    end
    validate_grammar_only(files[0])

  else
    $stderr.puts "Error: Unknown command '#{command}'"
    $stderr.puts
    print_usage
    2
  end
end

# ---------------------------------------------------------------------------
# main — parse argv with cli_builder, then dispatch
# ---------------------------------------------------------------------------

def main
  spec_path = File.join(ROOT, "code", "specs", "grammar-tools.json")

  begin
    parser = CLI::Parser.new(spec_path, [$0, *ARGV])
    result = parser.parse
  rescue CLI::ParseErrors => e
    e.errors.each { |err| $stderr.puts err.message }
    exit 2
  rescue StandardError => e
    $stderr.puts "Error: #{e.message}"
    exit 2
  end

  case result
  when CLI::HelpResult
    puts result.text
    return
  when CLI::VersionResult
    puts result.version
    return
  end

  # ParseResult
  args    = result.arguments
  command = args["command"].to_s
  files   = Array(args["files"])

  exit dispatch(command, files)
end

main if __FILE__ == $0
