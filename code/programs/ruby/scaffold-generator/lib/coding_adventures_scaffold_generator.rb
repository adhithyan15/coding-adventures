# frozen_string_literal: true

# coding_adventures_scaffold_generator.rb -- Entry point
# ======================================================
#
# === Load Order ===
#
# Ruby loads files in the order they are required. Dependencies MUST be
# required before our own modules, because our code may reference constants
# or classes defined in those dependencies. This is a critical convention
# documented in lessons.md.

# IMPORTANT: Require dependencies FIRST, before own modules.
require "coding_adventures_cli_builder"

require_relative "coding_adventures/scaffold_generator/version"
require_relative "coding_adventures/scaffold_generator/generator"

module CodingAdventures
  # ScaffoldGenerator -- Generate CI-ready package scaffolding for the
  # coding-adventures monorepo. Supports all six languages: Python, Go,
  # Ruby, TypeScript, Rust, and Elixir.
  module ScaffoldGenerator
  end
end
