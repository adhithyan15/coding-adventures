# frozen_string_literal: true

require_relative "lib/coding_adventures/virtual_machine/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_virtual_machine"
  spec.version       = CodingAdventures::VirtualMachine::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "General-purpose stack-based bytecode virtual machine"
  spec.description   = "A language-agnostic stack-based VM with trace recording. " \
                        "Supports arithmetic, variables, comparison, control flow, " \
                        "function calls, and I/O."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
