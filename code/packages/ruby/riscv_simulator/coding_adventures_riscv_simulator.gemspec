# frozen_string_literal: true

require_relative "lib/coding_adventures/riscv_simulator/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_riscv_simulator"
  spec.version = CodingAdventures::RiscvSimulator::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "RISC-V RV32I subset simulator"
  spec.description = "Simulates a subset of RISC-V RV32I: addi, add, sub, ecall. " \
    "Encodes/decodes real 32-bit RISC-V instructions."
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
