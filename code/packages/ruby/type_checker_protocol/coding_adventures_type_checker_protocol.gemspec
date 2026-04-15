# frozen_string_literal: true

require_relative "lib/coding_adventures_type_checker_protocol"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_type_checker_protocol"
  spec.version       = CodingAdventures::TypeCheckerProtocol::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Shared type-check result and hook-dispatch protocol for Ruby compiler frontends"
  spec.description   = "Provides TypeErrorDiagnostic, TypeCheckResult, and a small GenericTypeChecker hook dispatcher used by Ruby compiler frontends such as Nib."
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
