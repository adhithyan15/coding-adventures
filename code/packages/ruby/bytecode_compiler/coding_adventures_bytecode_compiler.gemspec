# frozen_string_literal: true

require_relative "lib/coding_adventures/bytecode_compiler/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_bytecode_compiler"
  spec.version = CodingAdventures::BytecodeCompiler::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Bytecode compiler with JVM, CLR, and WASM backends"
  spec.description = "Compiles AST to bytecode for our custom VM, the JVM, the CLR, " \
    "and WebAssembly. Four compiler backends from one AST."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_lexer"
  spec.add_dependency "coding_adventures_parser"
  spec.add_dependency "coding_adventures_virtual_machine"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
