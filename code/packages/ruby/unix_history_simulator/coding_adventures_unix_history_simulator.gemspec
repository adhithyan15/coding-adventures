# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_unix_history_simulator"
  spec.version = "0.1.0"
  spec.summary = "Reusable First Edition Unix filesystem and syscall teaching model"
  spec.authors = ["Coding Adventures contributors"]
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2"
  spec.files = ["lib/unix_history.rb", "exe/unix-history", "README.md", "CHANGELOG.md"]
  spec.bindir = "exe"
  spec.executables = ["unix-history"]
  spec.require_paths = ["lib"]
end
