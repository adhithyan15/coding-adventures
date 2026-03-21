# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_directed_graph_native"
  spec.version       = "0.1.0"
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Rust-backed directed graph with topological sort, cycle detection, and parallel execution levels"
  spec.description   = "A native Ruby extension wrapping a Rust directed graph library via Magnus. " \
                        "Provides the same API as the pure Ruby directed_graph gem but with " \
                        "significantly better performance on large graphs."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.1.0"

  spec.files         = Dir["lib/**/*.rb", "ext/**/*", "src/**/*", "Cargo.toml", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.extensions    = ["extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # rb_sys provides the Rust build toolchain integration for Ruby gems.
  # It handles finding Ruby headers, setting linker flags, and invoking
  # Cargo with the correct configuration.
  spec.add_dependency "rb_sys", "~> 0.9"

  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "rake-compiler", "~> 1.2"
end
