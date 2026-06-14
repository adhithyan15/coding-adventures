# frozen_string_literal: true

require_relative "lib/coding_adventures/interpreter_ir/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_interpreter_ir"
  spec.version = CodingAdventures::InterpreterIr::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Ruby LANG InterpreterIR data model"
  spec.description = "Portable InterpreterIR instructions, functions, modules, and feedback slots for the LANG VM chain."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
