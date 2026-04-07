# frozen_string_literal: true

# ================================================================
# coding_adventures_correlation_vector -- Top-Level Require File
# ================================================================
#
# This is the gem's entry point. When someone writes:
#
#   require "coding_adventures_correlation_vector"
#
# Ruby loads this file, which pulls in all dependencies and then
# our own modules in the correct order.
#
# Dependency loading order matters in Ruby! If you require your own
# code before its dependencies, you'll get NameError because the
# constants (CodingAdventures::Sha256, etc.) don't exist yet.
#
# Our dependency graph:
#
#   coding_adventures_sha256
#     -> CodingAdventures::Sha256.sha256_hex
#
#   coding_adventures_json_value  (transitively via json_serializer)
#     -> CodingAdventures::JsonValue.from_native
#
#   coding_adventures_json_serializer
#     -> CodingAdventures::JsonSerializer.serialize
#
# Then our own modules (version first, then the implementation that
# uses the dependencies):
#   1. version     -- gem version constant
#   2. correlation_vector -- the main implementation
# ================================================================

# Load dependencies FIRST before any of our own code.
# This is the critical require ordering lesson from lessons.md:
# "Ruby require ordering: dependencies must be required before own modules"
require "coding_adventures_sha256"
require "coding_adventures_json_serializer"

# Now load our own code in dependency order.
# version.rb must come before correlation_vector.rb because the
# gemspec references CodingAdventures::CorrelationVector::VERSION.
require_relative "coding_adventures/correlation_vector/version"
require_relative "coding_adventures/correlation_vector"

# ================================================================
# Convenience alias: CodingAdventures::JsonValue.from_native
# ================================================================
#
# The json_value gem exposes from_native at the module level
# as CodingAdventures::JsonValue.from_native. We rely on this
# in our serialize method to convert Ruby Hashes into JsonValue
# trees for our JsonSerializer to process.
#
# This module-level method is defined in the converter.rb of
# the json_value gem. We don't redefine it here -- just confirming
# that coding_adventures_json_value is already loaded transitively
# via coding_adventures_json_serializer above.
# ================================================================

module CodingAdventures
  # Nothing to define here -- the module is opened by the files above.
  # This block documents that CodingAdventures::CorrelationVector
  # is the public API surface.
end
