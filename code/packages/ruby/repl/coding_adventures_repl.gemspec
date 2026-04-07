# frozen_string_literal: true

require_relative "lib/coding_adventures/repl/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_repl"
  spec.version = CodingAdventures::Repl::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "A pluggable REPL framework with async eval and I/O injection."
  spec.description = <<~DESC
    A framework for building Read-Eval-Print Loops (REPLs) in Ruby. Provides
    three pluggable interfaces (Language, Prompt, Waiting) plus async eval
    via Thread and full I/O injection for testing. Ships with built-in
    implementations: EchoLanguage, DefaultPrompt, and SilentWaiting.
    No runtime dependencies — Ruby stdlib only.
  DESC
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1.0"

  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  # No runtime dependencies — uses only Ruby stdlib (thread is built in).

  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"

  spec.metadata["rubygems_mfa_required"] = "true"
end
