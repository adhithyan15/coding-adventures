# frozen_string_literal: true

# ============================================================================
# Version — single source of truth for the gem version
# ============================================================================
#
# Keeping the version in its own file means the gemspec can `require_relative`
# it without loading the entire library (and all its dependencies). This is
# the standard Ruby gem convention.

module CodingAdventures
  module Repl
    # Semantic version: MAJOR.MINOR.PATCH
    #   MAJOR — incompatible API changes
    #   MINOR — new backwards-compatible functionality
    #   PATCH — backwards-compatible bug fixes
    VERSION = "0.1.0"
  end
end
