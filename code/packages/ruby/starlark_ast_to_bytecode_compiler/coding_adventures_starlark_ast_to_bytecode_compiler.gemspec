# frozen_string_literal: true

require_relative "lib/coding_adventures/starlark_ast_to_bytecode_compiler/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_starlark_ast_to_bytecode_compiler"
  spec.version       = CodingAdventures::StarlarkAstToBytecodeCompiler::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Compiles Starlark AST to bytecode using the GenericCompiler framework"
  spec.description   = "Takes a Starlark AST (from starlark_parser) and compiles it into " \
                        "bytecode instructions (using bytecode_compiler's GenericCompiler and " \
                        "virtual_machine's types). Supports all Starlark language features: " \
                        "assignments, arithmetic, comparisons, control flow, functions, " \
                        "collections, load statements, and lambda expressions."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_bytecode_compiler", "~> 0.1"
  spec.add_dependency "coding_adventures_virtual_machine", "~> 0.1"
  spec.add_dependency "coding_adventures_starlark_lexer", "~> 0.1"
  spec.add_dependency "coding_adventures_starlark_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_lexer", "~> 0.1"
  spec.add_dependency "coding_adventures_grammar_tools", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
