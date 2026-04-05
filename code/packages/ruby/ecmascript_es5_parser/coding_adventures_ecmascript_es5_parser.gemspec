# frozen_string_literal: true

require_relative "lib/coding_adventures/ecmascript_es5_parser/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_ecmascript_es5_parser"
  spec.version       = CodingAdventures::EcmascriptEs5Parser::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Parses ECMAScript 5 (2009) source code into ASTs using a grammar-driven parser"
  spec.description   = "A thin wrapper around the grammar-driven parser engine that loads " \
                        "es5.grammar to parse ECMAScript 5 source code from Ruby. " \
                        "ES5 added debugger statement and getter/setter properties."
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
  spec.add_dependency "coding_adventures_lexer", "~> 0.1"
  spec.add_dependency "coding_adventures_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_ecmascript_es5_lexer", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
