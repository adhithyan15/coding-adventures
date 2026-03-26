# frozen_string_literal: true

require_relative "lib/coding_adventures/csv_parser/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_csv_parser"
  spec.version = CodingAdventures::CsvParser::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "CSV parser — hand-rolled state machine parser for RFC 4180-style CSV"
  spec.description = "Converts CSV text into an array of row hashes (column name => value). " \
                     "Implemented as a character-by-character state machine — no standard " \
                     "library CSV class used. All values are returned as strings."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
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
