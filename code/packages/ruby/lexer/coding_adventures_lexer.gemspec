# frozen_string_literal: true

require_relative "lib/coding_adventures/lexer/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_lexer"
  spec.version       = CodingAdventures::Lexer::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Tokenizer with hand-written and grammar-driven modes"
  spec.description   = "Breaks source code into tokens using either a hand-written lexer " \
                        "or a grammar-driven lexer that reads .tokens files."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_grammar_tools", "~> 0.1"
  spec.add_dependency "coding_adventures_state_machine", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
