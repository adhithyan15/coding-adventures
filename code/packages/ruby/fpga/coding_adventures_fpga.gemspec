# frozen_string_literal: true

require_relative "lib/coding_adventures/fpga/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_fpga"
  spec.version       = CodingAdventures::FPGA::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "FPGA fabric model — LUTs, slices, CLBs, switch matrices, I/O blocks, bitstream"
  spec.description   = "Layer 12 of the computing stack — a programmable FPGA fabric built on logic gates and block RAM."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Runtime dependencies
  spec.add_dependency "coding_adventures_logic_gates", "~> 0.1"
  spec.add_dependency "coding_adventures_block_ram", "~> 0.1"
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
