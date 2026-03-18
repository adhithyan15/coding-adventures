# frozen_string_literal: true

require_relative "lib/coding_adventures/clock/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_clock"
  spec.version       = CodingAdventures::Clock::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "System clock generator — the heartbeat of every digital circuit"
  spec.description   = "Simulates the crystal oscillator that drives all sequential logic in a computer."
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
