# frozen_string_literal: true

require_relative "lib/coding_adventures/bitset/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_bitset"
  spec.version       = CodingAdventures::Bitset::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "A compact bitset packed into 64-bit words with ArrayList-style growth"
  spec.description   = "A bitset data structure that packs boolean values into 64-bit integers, " \
                        "providing O(n/64) bulk bitwise operations (AND, OR, XOR, NOT), " \
                        "efficient iteration over set bits, and automatic capacity doubling."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
