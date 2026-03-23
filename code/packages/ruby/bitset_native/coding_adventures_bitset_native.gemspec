# frozen_string_literal: true

require_relative "lib/coding_adventures/bitset_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_bitset_native"
  spec.version       = CodingAdventures::BitsetNative::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Rust-backed bitset with compact 64-bit word storage and bulk bitwise operations"
  spec.description   = "A native extension wrapping the bitset Rust crate via ruby-bridge. " \
                        "Packs booleans into 64-bit words for 8x space savings and hardware-accelerated " \
                        "AND/OR/XOR/NOT/popcount operations."
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
  spec.extensions    = ["ext/bitset_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
