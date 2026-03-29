# frozen_string_literal: true

require_relative "lib/coding_adventures/logic_gates/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_logic_gates"
  spec.version       = CodingAdventures::LogicGates::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Fundamental logic gate implementations (AND, OR, NOT, XOR, NAND, NOR, XNOR)"
  spec.description   = "Layer 10 of the computing stack — the foundation of all digital logic."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "sig/**/*.rbs", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Runtime dependency: logic gates delegate CMOS evaluation to the transistors gem.
  # coding_adventures_transistors is monorepo-internal; downstream packages must
  # add `gem "coding_adventures_transistors", path: "../transistors"` to their
  # Gemfile so bundler resolves it from the local path rather than RubyGems.
  spec.add_dependency "coding_adventures_transistors", ">= 0.1.0"
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
