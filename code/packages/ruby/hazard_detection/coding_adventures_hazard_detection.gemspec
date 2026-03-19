# frozen_string_literal: true

require_relative "lib/coding_adventures/hazard_detection/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_hazard_detection"
  spec.version       = CodingAdventures::HazardDetection::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Pipeline hazard detection for a classic 5-stage CPU"
  spec.description   = "Detects data, control, and structural hazards in a pipelined CPU. Supports forwarding, stalling, and flushing."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
