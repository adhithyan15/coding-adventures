# frozen_string_literal: true

require_relative "lib/coding_adventures/block_ram/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_block_ram"
  spec.version       = CodingAdventures::BlockRam::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "SRAM cells, arrays, and RAM modules (single-port, dual-port, configurable BRAM)"
  spec.description   = "Layer 11 of the computing stack — block RAM for FPGAs and CPU caches."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Runtime dependency
  spec.add_dependency "coding_adventures_logic_gates", "~> 0.1"
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
