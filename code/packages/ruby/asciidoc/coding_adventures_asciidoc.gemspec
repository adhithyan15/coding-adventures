# frozen_string_literal: true

require_relative "lib/coding_adventures/asciidoc/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_asciidoc"
  spec.version = CodingAdventures::Asciidoc::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "AsciiDoc pipeline convenience package (parse + render)"
  spec.description = "Thin convenience wrapper combining coding_adventures_asciidoc_parser " \
                     "and coding_adventures_document_ast_to_html into a single pipeline. " \
                     "Exposes to_html(text). Part of the coding-adventures computing stack."
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
  spec.add_dependency "coding_adventures_asciidoc_parser", "~> 0.1"
  spec.add_dependency "coding_adventures_document_ast_to_html", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
