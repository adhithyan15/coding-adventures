# frozen_string_literal: true

require_relative "lib/coding_adventures/core/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_core"
  spec.version       = CodingAdventures::Core::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Configurable processor core simulator integrating pipeline, cache, branch predictor, and hazard detection"
  spec.description   = "Composes all D-series micro-architectural components (pipeline, branch predictor, hazard detection, cache hierarchy, register file) into a complete processor core. Supports multi-core configurations with shared memory."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Runtime dependencies
  spec.add_dependency "coding_adventures_cache", "~> 0.1"
  spec.add_dependency "coding_adventures_branch_predictor", "~> 0.1"
  spec.add_dependency "coding_adventures_cpu_pipeline", "~> 0.1"
  spec.add_dependency "coding_adventures_hazard_detection", "~> 0.1"
  spec.add_dependency "coding_adventures_clock", "~> 0.1"
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
