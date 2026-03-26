# frozen_string_literal: true

require_relative "lib/coding_adventures/sql_csv_source/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_sql_csv_source"
  spec.version       = CodingAdventures::SqlCsvSource::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "SQL CSV DataSource — connects the SQL execution engine to CSV files"
  spec.description   = "A thin adapter that implements the DataSource interface from " \
                       "coding_adventures_sql_execution_engine using csv_parser to read " \
                       "CSV files from a directory. Each tablename.csv in the directory " \
                       "is one queryable table with full type coercion."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri"    => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_csv_parser",           "~> 0.1"
  spec.add_dependency "coding_adventures_sql_execution_engine", "~> 0.1"
  spec.add_development_dependency "minitest",   "~> 5.0"
  spec.add_development_dependency "simplecov",  "~> 0.22"
  spec.add_development_dependency "rake",       "~> 13.0"
  spec.add_development_dependency "standard",   "~> 1.0"
end
