# frozen_string_literal: true

require_relative "lib/coding_adventures/wasm_module_encoder/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_wasm_module_encoder"
  spec.version       = CodingAdventures::WasmModuleEncoder::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "WebAssembly 1.0 module encoder"
  spec.description   = "Encodes CodingAdventures::WasmTypes::WasmModule values into raw WebAssembly bytes."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_wasm_leb128", "~> 0.1"
  spec.add_dependency "coding_adventures_wasm_types", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
