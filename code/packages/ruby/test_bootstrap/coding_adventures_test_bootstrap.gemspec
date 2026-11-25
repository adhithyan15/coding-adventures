# frozen_string_literal: true

require_relative "lib/coding_adventures/test_bootstrap/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_test_bootstrap"
  spec.version = CodingAdventures::TestBootstrap::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Deterministic property-based testing and coverage-guided fuzz engines (VER05h, VER06i)"
  spec.description = "SplitMix64-driven generators, integrated shrinking with a hierarchical " \
                     "work budget, an honest runner with a closed failure vocabulary, and a " \
                     "strict runner for the shared pbt-v1 conformance corpus; plus a coverage-guided " \
                     "fuzz engine (real in-process Coverage branch edges, budgeted handles, private " \
                     "reporting) and its fuzz-v1 corpus runner. Zero dependencies."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2.0"
  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.d/*.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "standard", "~> 1.0"
end
