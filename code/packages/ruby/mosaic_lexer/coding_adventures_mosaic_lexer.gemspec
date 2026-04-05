# frozen_string_literal: true

require_relative "lib/coding_adventures/mosaic_lexer/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_mosaic_lexer"
  spec.version       = CodingAdventures::MosaicLexer::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Tokenizes .mosaic source using the grammar-driven lexer"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }

  spec.add_dependency "coding_adventures_grammar_tools", "~> 0.1"
  spec.add_dependency "coding_adventures_lexer", "~> 0.1"
  spec.add_dependency "coding_adventures_directed_graph", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
