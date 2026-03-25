# frozen_string_literal: true

require_relative "lib/coding_adventures/document_html_sanitizer/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_document_html_sanitizer"
  spec.version = CodingAdventures::DocumentHtmlSanitizer::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Regex-based HTML string sanitizer — no DOM dependency"
  spec.description = "Sanitizes an HTML string by stripping dangerous elements, " \
                     "event handler attributes, unsafe URL schemes, and CSS expressions. " \
                     "String in, string out. No dependency on document-ast. " \
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
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
