# frozen_string_literal: true

require_relative "lib/coding_adventures/directed_graph/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_directed_graph"
  spec.version       = CodingAdventures::DirectedGraph::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Directed graph with topological sort, cycle detection, and parallel execution levels"
  spec.description   = "A directed graph library with Kahn's topological sort, cycle detection, " \
                        "transitive closure, and independent group computation for parallel builds."
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
