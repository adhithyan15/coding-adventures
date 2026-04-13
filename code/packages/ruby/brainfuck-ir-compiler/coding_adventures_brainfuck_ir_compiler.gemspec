# frozen_string_literal: true

require_relative "lib/coding_adventures/brainfuck_ir_compiler/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_brainfuck_ir_compiler"
  spec.version       = CodingAdventures::BrainfuckIrCompiler::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Brainfuck AOT compiler frontend — Brainfuck AST → IR"
  spec.description   = "The Brainfuck-specific frontend of the AOT compiler pipeline. " \
                        "Takes a Brainfuck AST (from coding_adventures_brainfuck) and emits " \
                        "a target-independent IrProgram (from coding_adventures_compiler_ir) " \
                        "plus the first two segments of the source map chain. Supports debug " \
                        "builds (bounds checks, debug locs) and release builds."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_compiler_ir", "~> 0.1"
  spec.add_dependency "coding_adventures_compiler_source_map", "~> 0.1"
  spec.add_dependency "coding_adventures_brainfuck", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
