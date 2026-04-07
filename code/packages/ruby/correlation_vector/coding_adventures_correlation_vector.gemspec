# frozen_string_literal: true

# ================================================================
# Gemspec -- Gem Specification for correlation_vector
# ================================================================
#
# A gemspec is Ruby's package manifest. It describes:
# - Who made the package and what version it is
# - What files to include in the distributed gem
# - What other gems it depends on (runtime and development)
#
# The gemspec uses `require_relative` to load our version constant.
# This happens at gem installation time, before any gems are loaded,
# so we must only use stdlib in gemspecs (no gem dependencies here).
# ================================================================

require_relative "lib/coding_adventures/correlation_vector/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_correlation_vector"
  spec.version = CodingAdventures::CorrelationVector::VERSION
  spec.authors = ["Coding Adventures"]
  spec.summary = "Correlation Vector — append-only provenance tracking for any entity"
  spec.description = <<~DESC
    A Correlation Vector (CV) is a lightweight, append-only provenance record
    that follows a piece of data through every transformation it undergoes.
    Assign a CV to any entity at birth; every system or stage that touches it
    appends its contribution. Domain-agnostic: useful for compiler pipelines,
    ETL workflows, build systems, ML preprocessing, and distributed tracing.
  DESC

  spec.homepage = "https://github.com/coding-adventures/coding-adventures"
  spec.license = "MIT"

  # Include all Ruby source files under lib/.
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  # The gem name that callers use in `require` statements.
  # By convention: underscores in gem name match underscores in require path.
  spec.required_ruby_version = ">= 3.2"

  # Runtime dependencies: these gems must be present when our gem is used.
  # We depend on our own SHA-256 and JSON serializer implementations as
  # a dog-fooding exercise (using our own packages end-to-end).
  spec.add_dependency "coding_adventures_sha256"
  spec.add_dependency "coding_adventures_json_serializer"

  # Development dependencies: only needed for testing and linting.
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
