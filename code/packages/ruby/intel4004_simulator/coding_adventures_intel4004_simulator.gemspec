# frozen_string_literal: true

require_relative "lib/coding_adventures/intel4004_simulator/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_intel4004_simulator"
  spec.version = CodingAdventures::Intel4004Simulator::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Intel 4004 accumulator machine simulator"
  spec.description = "Simulates the Intel 4004: LDM, XCH, ADD, SUB, HLT. " \
    "4-bit values, 16 registers, accumulator architecture."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
