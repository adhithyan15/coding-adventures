# frozen_string_literal: true

# This is the independent Ruby arithmetic foundation, not the optional Rust
# extension. An explicit file list keeps tests, coverage and local artifacts
# out of a distributable gem even when the checkout is dirty.
Gem::Specification.new do |spec|
  spec.name = "coding_adventures_matrix"
  spec.version = "0.2.0"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Independent pure-Ruby matrix arithmetic"
  spec.description = "Native Ruby rectangular matrix arithmetic, reductions and shape operations, with no runtime dependencies."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2"
  spec.files = %w[lib/matrix_ml.rb README.md CHANGELOG.md LICENSE required_capabilities.json]
  spec.require_paths = ["lib"]
  spec.metadata = {"source_code_uri" => spec.homepage, "rubygems_mfa_required" => "true"}
  spec.add_development_dependency "minitest", "~> 5.27"
  spec.add_development_dependency "rake", "~> 13.4"
  spec.add_development_dependency "simplecov", "~> 0.22.0"
  spec.add_development_dependency "standard", "~> 1.56.0"
end
