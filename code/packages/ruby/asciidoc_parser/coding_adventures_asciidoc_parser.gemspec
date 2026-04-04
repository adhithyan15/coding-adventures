# frozen_string_literal: true

require_relative "lib/coding_adventures/asciidoc_parser/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_asciidoc_parser"
  spec.version = CodingAdventures::AsciidocParser::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "AsciiDoc parser that produces Document AST"
  spec.description = "Parses AsciiDoc source text into a format-agnostic Document AST " \
                     "(coding_adventures_document_ast). Block parser state machine + inline " \
                     "character scanner. Part of the coding-adventures computing stack."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2.0"
  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_document_ast", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "coding_adventures_document_ast_to_html"
end
