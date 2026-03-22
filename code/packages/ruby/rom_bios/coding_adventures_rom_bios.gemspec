# frozen_string_literal: true

require_relative "lib/coding_adventures/rom_bios/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_rom_bios"
  spec.version = CodingAdventures::RomBios::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "ROM & BIOS firmware for simulated computer power-on"
  spec.description = "Implements ROM (read-only memory) and BIOS firmware generator " \
    "that produces RISC-V machine code for hardware initialization."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_riscv_simulator", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
