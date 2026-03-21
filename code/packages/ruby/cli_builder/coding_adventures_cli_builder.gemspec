# frozen_string_literal: true

require_relative "lib/coding_adventures/cli_builder/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_cli_builder"
  spec.version       = CodingAdventures::CliBuilder::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Declarative CLI argument parsing driven by directed graphs and state machines"
  spec.description   = "A runtime library for building CLI tools from a JSON specification file. " \
                       "CLI Builder separates what a tool accepts (the spec) from what it does " \
                       "(the implementation). Uses directed graphs for command routing and state " \
                       "machines for parsing. Part of the coding-adventures computing stack."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Runtime dependencies
  spec.add_dependency "coding_adventures_state_machine", "~> 0.1"
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
