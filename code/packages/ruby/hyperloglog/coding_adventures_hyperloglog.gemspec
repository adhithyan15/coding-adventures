# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_hyperloglog"
  spec.version = "0.1.0"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "A small HyperLogLog cardinality estimator"
  spec.description = "A dependency-free HyperLogLog implementation for approximate distinct counting and merges."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 2.6.0"
  spec.files = Dir["lib/**/*.rb", "test/**/*.rb"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
