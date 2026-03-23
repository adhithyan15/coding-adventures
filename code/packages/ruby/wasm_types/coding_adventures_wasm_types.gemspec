# frozen_string_literal: true

require_relative "lib/coding_adventures/wasm_types/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_wasm_types"
  spec.version       = CodingAdventures::WasmTypes::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "WASM 1.0 type system: ValueType, FuncType, Limits, MemoryType, TableType, GlobalType, WasmModule"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }

  spec.add_dependency "coding_adventures_wasm_leb128", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
