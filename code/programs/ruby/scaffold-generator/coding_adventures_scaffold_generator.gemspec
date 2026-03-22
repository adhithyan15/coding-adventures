# frozen_string_literal: true

require_relative "lib/coding_adventures/scaffold_generator/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_scaffold_generator"
  spec.version       = CodingAdventures::ScaffoldGenerator::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Generate CI-ready package scaffolding for the coding-adventures monorepo"
  spec.description   = <<~DESC
    A CLI tool powered by cli-builder that generates correctly-structured,
    CI-ready package directories for all six languages (Python, Go, Ruby,
    TypeScript, Rust, Elixir) in the coding-adventures monorepo.
  DESC
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_cli_builder", "~> 0.1"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
end
