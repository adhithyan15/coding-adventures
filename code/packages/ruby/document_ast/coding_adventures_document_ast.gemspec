# frozen_string_literal: true

require_relative "lib/coding_adventures/document_ast/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_document_ast"
  spec.version = CodingAdventures::DocumentAst::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Format-agnostic Document AST — the IR between document parsers and renderers"
  spec.description = "The Document AST is the LLVM IR of documents — a stable, typed, immutable " \
                       "tree that every front-end parser produces and every back-end renderer " \
                       "consumes. Supports Markdown, RST, HTML, DOCX input and HTML, PDF, LaTeX, " \
                       "plain text output. Part of the coding-adventures computing stack."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2.0"
  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # No runtime dependencies — this is a types-only package
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
