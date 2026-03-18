# frozen_string_literal: true

require_relative "lib/coding_adventures/arm_simulator/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_arm_simulator"
  spec.version = CodingAdventures::ArmSimulator::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "ARMv7 subset simulator with encoding/decoding"
  spec.description = "Simulates a subset of ARMv7: MOV immediate, ADD register, " \
    "SUB register, and HLT. Encodes/decodes real 32-bit ARM instructions."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_cpu_simulator", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
