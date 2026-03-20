# frozen_string_literal: true

# ============================================================================
# CA::CapabilityAnalyzer — Static Capability Analyzer for Ruby
# ============================================================================
#
# This package walks Ruby ASTs to detect OS capability usage and banned
# dynamic execution constructs. It answers two questions:
#
#   1. "What OS resources does this Ruby code touch?"
#      (filesystem, network, processes, environment variables, FFI)
#
#   2. "Does this code use constructs that evade static analysis?"
#      (eval, send with dynamic args, backticks, method_missing, etc.)
#
# ## How It Works
#
# We use the Prism parser (Ruby's official parser, available as a gem)
# to parse Ruby source into an AST. Then we walk the tree looking for
# patterns that indicate capability usage:
#
#   - `require "socket"` implies network access
#   - `File.open("x")` implies filesystem read
#   - `system("cmd")` implies process execution
#   - `ENV["KEY"]` implies environment variable reading
#
# The results are compared against a `required_capabilities.json` manifest
# to enforce a "default deny" security policy: if a package doesn't
# declare a capability, using it is an error.
#
# ## Architecture
#
#   ca_capability_analyzer.rb  (this file — entry point)
#   ca/capability_analyzer/
#     version.rb               — gem version
#     analyzer.rb              — AST walker for capability detection
#     banned.rb                — AST walker for banned construct detection
#     manifest.rb              — manifest loading and comparison
#     cli.rb                   — command-line interface
# ============================================================================

require_relative "ca/capability_analyzer/version"
require_relative "ca/capability_analyzer/analyzer"
require_relative "ca/capability_analyzer/banned"
require_relative "ca/capability_analyzer/manifest"
require_relative "ca/capability_analyzer/cli"
