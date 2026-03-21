# frozen_string_literal: true

require_relative "lib/coding_adventures/directed_graph_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_directed_graph_native"
  spec.version       = CodingAdventures::DirectedGraphNative::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Rust-backed directed graph with topological sort, cycle detection, and parallel execution levels"
  spec.description   = "A native extension wrapping the directed-graph Rust crate via ruby-bridge. " \
                        "Same API as the pure Ruby directed_graph gem but backed by Rust for performance."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"

  spec.files         = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "README.md",
    "CHANGELOG.md",
  ]
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/directed_graph_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
