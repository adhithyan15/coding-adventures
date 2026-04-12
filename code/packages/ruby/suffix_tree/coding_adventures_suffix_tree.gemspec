# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_suffix_tree"
  spec.version = "0.1.0"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "A suffix tree for substring search"
  spec.description = "A Ruby suffix tree facade for substring search and repeated substring queries."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.0.0"
  spec.files = Dir["lib/**/*.rb", "test/**/*.rb"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
