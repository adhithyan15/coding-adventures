# frozen_string_literal: true

require_relative "lib/coding_adventures/haskell_lexer/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_haskell_lexer"
  spec.version       = CodingAdventures::HaskellLexer::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Tokenizes Haskell source code using a grammar-driven lexer"
  spec.description   = "A thin wrapper around the grammar-driven lexer engine that loads " \
                        "haskell/haskell<version>.tokens to tokenize Haskell source code from Ruby. " \
                        "Demonstrates cross-language grammar reuse."
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
