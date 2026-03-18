# frozen_string_literal: true

require_relative "lib/coding_adventures/arithmetic/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_arithmetic"
  spec.version = CodingAdventures::Arithmetic::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Arithmetic circuits — Layer 9 of the coding-adventures computing stack."
  spec.description = <<~DESC
    Implements half adder, full adder, ripple-carry adder, and an N-bit ALU
    built entirely from logic gates. Supports ADD, SUB, AND, OR, XOR, and NOT
    operations with zero, carry, negative, and overflow status flags.
    This is Layer 9 (Arithmetic) of the computing stack, built on Layer 10
    (Logic Gates).
  DESC
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1.0"

  spec.files = Dir["lib/**/*.rb", "sig/**/*.rbs", "README.md", "CHANGELOG.md", "LICENSE.txt"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_logic_gates", "~> 0.1"

  spec.metadata["rubygems_mfa_required"] = "true"
end
