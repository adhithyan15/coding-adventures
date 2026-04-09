# frozen_string_literal: true

require_relative "lib/coding_adventures/register_vm/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_register_vm"
  spec.version       = CodingAdventures::RegisterVM::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Generic register-based virtual machine with accumulator model and feedback vectors"
  spec.description   = "Implements the V8 Ignition execution model for educational purposes. " \
                       "Features: accumulator register, register file per call frame, " \
                       "per-function feedback vectors for JIT-readiness, ~70 opcodes."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.1.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri"       => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
end
