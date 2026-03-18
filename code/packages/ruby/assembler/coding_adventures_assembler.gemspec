# frozen_string_literal: true

require_relative "lib/coding_adventures/assembler/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_assembler"
  spec.version = CodingAdventures::Assembler::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Assembler for ARM instruction set (shell gem)"
  spec.description = "Shell gem for the assembler package. Implementation forthcoming."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_arm_simulator"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
