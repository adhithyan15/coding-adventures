# frozen_string_literal: true

require_relative "lib/coding_adventures/matrix_rust_ruby/version"

# coding_adventures_matrix_rust_ruby — Ruby gem that drives the Rust
# matrix-cpu execution engine.
#
# The gem ships with a Rust native extension (matrix_rust_ruby_native, the
# workspace crate that delegates to c-bridge's pure-Rust envelope runner).
# Users get one entry point:
#
#   MatrixRustRuby.run_graph_on_cpu(envelope_json_str) -> envelope_json_str
#
# This gem is the "low-level binding" tier of the multi-language plan.  On
# top of it sits ml_framework_core for Ruby (the idiomatic Tensor + autograd
# layer; PRs #4-#8).
Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_matrix_rust_ruby"
  spec.version       = CodingAdventures::MatrixRustRuby::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Ruby bindings for the Rust matrix-cpu execution engine"
  spec.description   = "Drive the Rust matrix-cpu graph executor from Ruby via a JSON envelope. " \
                       "Wraps the matrix_rust_ruby_native workspace crate, which itself delegates " \
                       "to c-bridge's pure-Rust run_graph_on_cpu_via_json_envelope."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 2.6.0"

  # Ship Ruby sources + the extconf scaffolding.  The actual Rust source is
  # not packaged with the gem — when users install from RubyGems we'll need
  # a published artifact (cross-compiled per-platform .gem); for now this
  # gem builds from-source against the workspace Rust crate via path.
  spec.files         = Dir[
    "lib/**/*.rb",
    "ext/**/*.rb",
    "README.md",
    "CHANGELOG.md"
  ]
  spec.require_paths = ["lib"]

  # extconf.rb shells out to `cargo build -p matrix-rust-ruby-native --release`,
  # then copies the resulting dylib/so/dll into lib/coding_adventures/matrix_rust_ruby/.
  # See ext/matrix_rust_ruby_native/extconf.rb for the full dance.
  spec.extensions    = ["ext/matrix_rust_ruby_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
