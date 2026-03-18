# frozen_string_literal: true

require_relative "lib/coding_adventures/pipeline/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_pipeline"
  spec.version = CodingAdventures::Pipeline::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Pipeline orchestrator chaining lexer, parser, compiler, and VM"
  spec.description = "Orchestrates the full computing stack: source -> lexer -> parser -> " \
    "compiler -> VM. Captures traces at every stage for visualization."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_lexer"
  spec.add_dependency "coding_adventures_parser"
  spec.add_dependency "coding_adventures_bytecode_compiler"
  spec.add_dependency "coding_adventures_virtual_machine"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
