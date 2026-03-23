# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_tree"
  spec.version       = "0.1.0"
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "A rooted tree data structure backed by a directed graph"
  spec.description   = "A rooted tree library with traversals (preorder, postorder, level-order), " \
                        "lowest common ancestor, subtree extraction, and ASCII visualization. " \
                        "Built on top of the coding_adventures_directed_graph library."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # Runtime dependency: we build the tree ON TOP of the directed graph
  spec.add_dependency "coding_adventures_directed_graph"

  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
