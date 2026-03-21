# frozen_string_literal: true

require_relative "lib/coding_adventures/gpu_core/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_gpu_core"
  spec.version       = CodingAdventures::GpuCore::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "A generic, pluggable GPU processing element simulator"
  spec.description   = "Simulates a single GPU core with configurable registers, " \
                        "local memory, and a pluggable instruction set architecture (ISA). " \
                        "Built on IEEE 754 floating-point arithmetic from logic gates."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_fp_arithmetic"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
