# frozen_string_literal: true

require_relative "lib/ca/capability_analyzer/version"

Gem::Specification.new do |spec|
  spec.name = "ca_capability_analyzer"
  spec.version = CA::CapabilityAnalyzer::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Static capability analyzer for Ruby source code."
  spec.description = <<~DESC
    Walks Ruby ASTs (via the Prism parser) to detect OS capability usage
    — filesystem, network, process, environment, and FFI access — and
    banned dynamic execution constructs (eval, send with dynamic args,
    backticks, etc.). Compares detected capabilities against a declared
    manifest for CI gating.
  DESC
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"

  spec.files = Dir["lib/**/*.rb", "exe/*", "README.md", "CHANGELOG.md", "LICENSE.txt"]
  spec.bindir = "exe"
  spec.executables = ["ca-capability-analyzer"]
  spec.require_paths = ["lib"]

  # ── Runtime dependency ────────────────────────────────────────────
  # Prism is Ruby's official parser, shipping as a gem since Ruby 3.3.
  # We use it to parse Ruby source into an AST without invoking eval
  # or the built-in parser (which can execute code in edge cases).
  spec.add_dependency "prism", ">= 0.24"

  # ── Development dependencies ─────────────────────────────────────
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"

  spec.metadata["rubygems_mfa_required"] = "true"
end
