# frozen_string_literal: true

require_relative "lib/coding_adventures/tree_set/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_tree_set"
  spec.version = CodingAdventures::TreeSet::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "An ordered tree set with sorted iteration and set algebra"
  spec.description = "An ordered set backed by the AVL tree package with rank, range, and set algebra helpers."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.0.0"
  spec.files = Dir["lib/**/*.rb", "test/**/*.rb"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_avl_tree"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
