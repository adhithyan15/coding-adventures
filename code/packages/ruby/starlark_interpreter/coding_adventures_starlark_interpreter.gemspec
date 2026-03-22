# frozen_string_literal: true

require_relative "lib/coding_adventures/starlark_interpreter/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_starlark_interpreter"
  spec.version       = CodingAdventures::StarlarkInterpreter::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Full Starlark interpreter with load() support"
  spec.description   = "Chains the entire Starlark pipeline (lexer, parser, compiler, VM) " \
                        "into a single interpreter with load() statement support. " \
                        "Provides file_resolver-based module loading, caching of loaded " \
                        "modules, and convenience methods for interpreting source strings " \
                        "and files."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_starlark_vm", "~> 0.1"
  spec.add_dependency "coding_adventures_starlark_ast_to_bytecode_compiler", "~> 0.1"
  spec.add_dependency "coding_adventures_virtual_machine", "~> 0.1"
  spec.add_dependency "coding_adventures_bytecode_compiler", "~> 0.1"
  spec.add_dependency "coding_adventures_starlark_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_starlark_lexer", "~> 0.1"
  spec.add_dependency "coding_adventures_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_lexer", "~> 0.1"
  spec.add_dependency "coding_adventures_grammar_tools", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
