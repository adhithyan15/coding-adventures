# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_neural_framework"
  spec.version = "0.1.0"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Independent native Ruby dense neural training"
  spec.description = "Owned dense models, analytical gradients, transactional SGD and portable V1 checkpoints above the pure-Ruby matrix foundation."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2"
  spec.files = %w[lib/neural_framework.rb examples/xor.rb README.md CHANGELOG.md LICENSE required_capabilities.json]
  spec.require_paths = ["lib"]
  spec.metadata = {"source_code_uri" => spec.homepage, "rubygems_mfa_required" => "true"}
  spec.add_dependency "coding_adventures_matrix", "~> 0.2.0"
  spec.add_development_dependency "minitest", "~> 5.27"
  spec.add_development_dependency "rake", "~> 13.4"
  spec.add_development_dependency "simplecov", "~> 0.22.0"
  spec.add_development_dependency "standard", "~> 1.56.0"
end
