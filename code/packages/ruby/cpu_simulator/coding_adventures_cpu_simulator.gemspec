# frozen_string_literal: true

require_relative "lib/coding_adventures/cpu_simulator/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_cpu_simulator"
  spec.version = CodingAdventures::CpuSimulator::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Generic CPU simulator with fetch-decode-execute cycle"
  spec.description = "A generic CPU simulator providing registers, memory, " \
    "program counter, and the fetch-decode-execute pipeline. " \
    "ISA simulators (ARM, RISC-V, etc.) plug in their own decoders."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
