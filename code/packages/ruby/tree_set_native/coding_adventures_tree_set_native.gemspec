# frozen_string_literal: true

require_relative "lib/coding_adventures/tree_set_native/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_tree_set_native"
  spec.version = CodingAdventures::TreeSetNative::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Rust-backed tree set for Ruby"
  spec.description = "A native extension wrapping the Rust tree-set crate via ruby-bridge with sorted iteration, rank, range, and set algebra helpers."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.0.0"

  spec.files = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "README.md",
    "CHANGELOG.md",
  ]
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/tree_set_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
