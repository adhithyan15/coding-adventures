# frozen_string_literal: true

require_relative "lib/coding_adventures/nib_type_checker/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_nib_type_checker"
  spec.version       = CodingAdventures::NibTypeChecker::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Nib semantic checker for Ruby"
  spec.description   = "Type-checks the convergence-wave Nib subset and returns a typed AST wrapper for later stages."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_nib_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_type_checker_protocol", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
