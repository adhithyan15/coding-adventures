# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_avl_tree"
  spec.version = "0.1.0"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "A self-balancing AVL tree"
  spec.description = "A Ruby AVL tree with insertion, deletion, and order-statistics helpers."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.0.0"
  spec.files = Dir["lib/**/*.rb", "test/**/*.rb"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_binary_search_tree"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
