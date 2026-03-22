# frozen_string_literal: true

require_relative "lib/coding_adventures/transistors/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_transistors"
  spec.version       = CodingAdventures::Transistors::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Transistor-level circuit simulation: MOSFETs, BJTs, CMOS/TTL gates, amplifiers"
  spec.description   = "Layer 11 of the computing stack — transistor physics and circuit simulation."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "sig/**/*.rbs", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
