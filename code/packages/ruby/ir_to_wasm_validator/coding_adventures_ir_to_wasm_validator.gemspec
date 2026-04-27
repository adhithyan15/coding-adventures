# frozen_string_literal: true

require_relative "lib/coding_adventures/ir_to_wasm_validator/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_ir_to_wasm_validator"
  spec.version       = CodingAdventures::IrToWasmValidator::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Validator for IR-to-WASM lowering compatibility"
  spec.description   = "Runs the IR-to-WASM compiler in validation mode and reports lowering errors without requiring a full packaging step."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_ir_to_wasm_compiler", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
