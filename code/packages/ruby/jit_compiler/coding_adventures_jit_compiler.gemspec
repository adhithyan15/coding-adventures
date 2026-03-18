# frozen_string_literal: true

require_relative "lib/coding_adventures/jit_compiler/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_jit_compiler"
  spec.version = CodingAdventures::JitCompiler::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "JIT compiler for the virtual machine (shell gem)"
  spec.description = "Shell gem for the JIT compiler package. Implementation forthcoming."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_virtual_machine"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
