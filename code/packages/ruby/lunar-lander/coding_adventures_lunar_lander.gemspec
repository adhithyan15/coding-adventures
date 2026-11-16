# frozen_string_literal: true

require_relative "lib/coding_adventures/lunar_lander/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_lunar_lander"
  spec.version       = CodingAdventures::LunarLander::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Pure, deterministic simulation core and ASCII telemetry for Lunar Lander (GAME02)"
  spec.description   = "Fixed-point discrete Apollo Lunar Module descent simulation core with 26-CWE weakness closure and declarative ASCII telemetry."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
end
