# frozen_string_literal: true

require_relative "lib/coding_adventures/wasm_simulator/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_wasm_simulator"
  spec.version = CodingAdventures::WasmSimulator::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "WebAssembly stack machine simulator"
  spec.description = "Simulates a subset of WebAssembly: i32.const, i32.add, " \
    "i32.sub, local.get, local.set, end. Variable-width encoding."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
