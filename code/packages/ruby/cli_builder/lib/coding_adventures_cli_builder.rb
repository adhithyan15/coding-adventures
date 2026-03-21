# frozen_string_literal: true

# Entry point for the coding_adventures_cli_builder gem.
#
# CLI Builder is a runtime library for declarative CLI argument parsing,
# driven by directed graphs and state machines.
#
# === Architecture Overview ===
#
# A developer writes a JSON "spec file" describing their CLI tool's structure:
# what flags it accepts, what subcommands it has, what positional arguments
# each command expects. CLI Builder reads this file at startup, validates it,
# builds internal data structures, and handles all parsing, validation, help
# generation, and error messaging.
#
# The developer focuses entirely on what the tool does — not how to parse input.
#
# === Key Abstractions ===
#
# **DirectedGraph (G_cmd)** — The command tree is a directed graph where each
# node is a command and each edge is labeled by the subcommand token. Routing
# traverses this graph to find the active context.
#
# **DirectedGraph (G_flag)** — Flag dependency constraints ("A requires B") also
# form a directed graph. Cycle detection catches contradictory specs at load time.
# Transitive closure enforces transitive requirements at parse time.
#
# **ModalStateMachine** — Parsing is stateful. After "--", all tokens are
# positional. After "--output", the next token is a value. The Modal State
# Machine tracks these modes explicitly, making the parser logic a clean
# match-on-mode dispatch rather than tangled if-else chains.
#
# **TokenClassifier** — Classifies a single argv token (e.g. "-lah", "--output=file")
# into a typed event for the state machine. Implements the longest-match-first
# disambiguation algorithm.
#
# === Usage ===
#
#   require "coding_adventures_cli_builder"
#
#   begin
#     result = CodingAdventures::CliBuilder::Parser.new("my-tool.json", ARGV).parse
#     case result
#     when CodingAdventures::CliBuilder::ParseResult
#       run(result.flags, result.arguments)
#     when CodingAdventures::CliBuilder::HelpResult
#       puts result.text
#       exit 0
#     when CodingAdventures::CliBuilder::VersionResult
#       puts result.version
#       exit 0
#     end
#   rescue CodingAdventures::CliBuilder::ParseErrors => e
#     e.errors.each { |err| warn err.message }
#     exit 1
#   rescue CodingAdventures::CliBuilder::SpecError => e
#     warn "CLI spec error: #{e.message}"
#     exit 2
#   end

require "set"
require "coding_adventures_state_machine"

require_relative "coding_adventures/cli_builder/version"
require_relative "coding_adventures/cli_builder/errors"
require_relative "coding_adventures/cli_builder/types"
require_relative "coding_adventures/cli_builder/spec_loader"
require_relative "coding_adventures/cli_builder/token_classifier"
require_relative "coding_adventures/cli_builder/positional_resolver"
require_relative "coding_adventures/cli_builder/flag_validator"
require_relative "coding_adventures/cli_builder/help_generator"
require_relative "coding_adventures/cli_builder/parser"
