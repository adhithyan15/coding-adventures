# frozen_string_literal: true

require_relative "lib/coding_adventures/intel4004_gatelevel/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_intel4004_gatelevel"
  spec.version = CodingAdventures::Intel4004Gatelevel::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Intel 4004 gate-level simulator -- all operations route through real logic gates."
  spec.description = <<~DESC
    A gate-level simulator for the Intel 4004 microprocessor where every computation
    routes through real logic gates (NOT, AND, OR, XOR), flip-flops, and adder circuits.
    No behavioral shortcuts. Built on the coding_adventures_logic_gates and
    coding_adventures_arithmetic packages.
  DESC
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1.0"

  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_logic_gates", "~> 0.1"
  spec.add_dependency "coding_adventures_arithmetic", "~> 0.1"

  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "minitest", "~> 5.0"

  spec.metadata["rubygems_mfa_required"] = "true"
end
