# frozen_string_literal: true

require_relative "lib/coding_adventures/actor/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_actor"
  spec.version       = CodingAdventures::Actor::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Actor model — messages, channels, and actors for concurrent computation"
  spec.description   = "Implements the Actor model with three primitives: Message (immutable binary-native data), Channel (one-way append-only persistent log), and Actor (isolated computation with mailbox and behavior function)."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
