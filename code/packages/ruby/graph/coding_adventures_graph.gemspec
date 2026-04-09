# frozen_string_literal: true

require_relative "lib/coding_adventures/graph/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_graph"
  spec.version       = CodingAdventures::Graph::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Undirected graph data structure from scratch"
  spec.description   = "An undirected graph library implementation from scratch."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
