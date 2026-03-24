# frozen_string_literal: true

require_relative "lib/coding_adventures/commonmark_parser/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_commonmark_parser"
  spec.version = CodingAdventures::CommonmarkParser::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "CommonMark 0.31.2 compliant Markdown parser producing Document AST"
  spec.description = "Parses CommonMark 0.31.2 Markdown into a format-agnostic Document AST " \
                       "(coding_adventures_document_ast). Passes all 652 CommonMark spec tests. " \
                       "Two-phase parser: block structure (Phase 1) + inline content (Phase 2). " \
                       "Part of the coding-adventures computing stack."
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
  spec.add_dependency "coding_adventures_state_machine", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
