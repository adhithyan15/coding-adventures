# frozen_string_literal: true

require_relative "lib/coding_adventures/sql_execution_engine/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_sql_execution_engine"
  spec.version       = CodingAdventures::SqlExecutionEngine::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "SQL execution engine — executes SELECT queries against pluggable data sources"
  spec.description   = "A SELECT-only SQL execution engine built on top of the grammar-driven " \
                        "sql-parser package. Implements the full relational pipeline: FROM, JOIN, " \
                        "WHERE, GROUP BY, HAVING, SELECT, DISTINCT, ORDER BY, LIMIT/OFFSET."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_sql_parser", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
