# frozen_string_literal: true

require_relative "lib/coding_adventures/nib_parser/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_nib_parser"
  spec.version       = CodingAdventures::NibParser::VERSION
  spec.authors       = ["coding-adventures"]
  spec.email         = ["noreply@example.com"]
  spec.summary       = "Grammar-driven Nib parser"
  spec.description   = "Parses Nib source using the shared grammar-driven parser engine."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "required_capabilities.json"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_runtime_dependency "coding_adventures_grammar_tools"
  spec.add_runtime_dependency "coding_adventures_parser"
  spec.add_runtime_dependency "coding_adventures_nib_lexer"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
