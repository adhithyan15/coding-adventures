# frozen_string_literal: true

require_relative "lib/coding_adventures/ecmascript_es1_lexer/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_ecmascript_es1_lexer"
  spec.version       = CodingAdventures::EcmascriptEs1Lexer::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Tokenizes ECMAScript 1 (1997) source code using a grammar-driven lexer"
  spec.description   = "A thin wrapper around the grammar-driven lexer engine that loads " \
                        "es1.tokens to tokenize ECMAScript 1 source code from Ruby. " \
                        "ES1 was the first standardized version of JavaScript (ECMA-262, 1st Edition)."
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
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
