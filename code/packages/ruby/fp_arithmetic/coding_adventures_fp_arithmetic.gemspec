# frozen_string_literal: true

require_relative "lib/coding_adventures/fp_arithmetic/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_fp_arithmetic"
  spec.version       = CodingAdventures::FpArithmetic::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "IEEE 754 floating-point arithmetic built from logic gates"
  spec.description   = "Layer 30 of the computing stack — FP32/FP16/BF16 encoding, " \
                        "addition, multiplication, FMA, and pipelined FP units."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_logic_gates"
  spec.add_dependency "coding_adventures_arithmetic"
  spec.add_dependency "coding_adventures_clock"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
