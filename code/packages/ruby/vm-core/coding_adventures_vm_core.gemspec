# frozen_string_literal: true

require_relative "lib/coding_adventures/vm_core/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_vm_core"
  spec.version = CodingAdventures::VmCore::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Ruby LANG VM core"
  spec.description = "A language-agnostic register VM for InterpreterIR, with profiling, branch metrics, and JIT hooks."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_interpreter_ir", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
