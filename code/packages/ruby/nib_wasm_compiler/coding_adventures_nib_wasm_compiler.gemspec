# frozen_string_literal: true

require_relative "lib/coding_adventures/nib_wasm_compiler/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_nib_wasm_compiler"
  spec.version       = CodingAdventures::NibWasmCompiler::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Nib source to WebAssembly compiler"
  spec.description   = "Packages the Ruby Nib frontend lane with the existing generic IR-to-WASM backend."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_nib_ir_compiler", "~> 0.1"
  spec.add_dependency "coding_adventures_nib_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_nib_type_checker", "~> 0.1"
  spec.add_dependency "coding_adventures_ir_to_wasm_compiler", "~> 0.1"
  spec.add_dependency "coding_adventures_ir_to_wasm_validator", "~> 0.1"
  spec.add_dependency "coding_adventures_wasm_module_encoder", "~> 0.1"
  spec.add_dependency "coding_adventures_wasm_validator", "~> 0.1"
  spec.add_development_dependency "coding_adventures_wasm_runtime", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
