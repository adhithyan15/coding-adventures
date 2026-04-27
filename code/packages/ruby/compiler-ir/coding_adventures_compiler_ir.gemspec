# frozen_string_literal: true

require_relative "lib/coding_adventures/compiler_ir/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_compiler_ir"
  spec.version       = CodingAdventures::CompilerIr::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "General-purpose IR type library for the AOT compiler pipeline"
  spec.description   = "Defines the intermediate representation (IR) used by the AOT native " \
                        "compiler pipeline: opcodes, operand types, instruction structs, " \
                        "data declarations, IR programs, a text printer, and a text parser " \
                        "for roundtrip fidelity. Version 1 covers the Brainfuck subset."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
