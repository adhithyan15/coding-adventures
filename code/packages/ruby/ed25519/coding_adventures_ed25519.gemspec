# frozen_string_literal: true

require_relative "lib/coding_adventures_ed25519/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_ed25519"
  spec.version       = CodingAdventures::Ed25519::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Ed25519 digital signatures (RFC 8032) — from-scratch elliptic curve arithmetic over GF(2^255-19)"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_sha512", ">= 0.1.0"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
