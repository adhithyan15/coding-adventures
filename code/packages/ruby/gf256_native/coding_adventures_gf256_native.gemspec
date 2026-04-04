# frozen_string_literal: true

require_relative "lib/coding_adventures/gf256_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_gf256_native"
  spec.version       = CodingAdventures::GF256Native::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Rust-backed GF(2^8) finite field arithmetic for Reed-Solomon and AES"
  spec.description   = "A native extension wrapping the gf256 Rust crate via ruby-bridge. " \
                        "Provides add, subtract, multiply, divide, power, and inverse operations " \
                        "in GF(2^8) using log/antilog tables for O(1) multiplication. " \
                        "Used as the arithmetic foundation for Reed-Solomon error correction."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"

  spec.files         = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "README.md",
    "CHANGELOG.md",
  ]
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/gf256_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
