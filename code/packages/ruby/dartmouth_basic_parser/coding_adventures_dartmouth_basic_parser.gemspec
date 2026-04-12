# frozen_string_literal: true

require_relative "lib/coding_adventures/dartmouth_basic_parser/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_dartmouth_basic_parser"
  spec.version       = CodingAdventures::DartmouthBasicParser::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Parses 1964 Dartmouth BASIC source code into ASTs using a grammar-driven parser"
  spec.description   = "A thin wrapper around the grammar-driven parser engine that loads " \
                        "dartmouth_basic.grammar to parse Dartmouth BASIC 1964 source code from Ruby. " \
                        "Supports all 17 statement types and the full expression precedence hierarchy."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_grammar_tools", "~> 0.1"
  spec.add_dependency "coding_adventures_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_dartmouth_basic_lexer", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
