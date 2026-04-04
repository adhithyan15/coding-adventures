# frozen_string_literal: true

require_relative "lib/coding_adventures/polynomial_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_polynomial_native"
  spec.version       = CodingAdventures::PolynomialNative::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Rust-backed polynomial arithmetic over f64 coefficient arrays"
  spec.description   = "A native extension wrapping the polynomial Rust crate via ruby-bridge. " \
                        "Provides normalize, degree, add, subtract, multiply, divmod, divide, " \
                        "modulo, evaluate, and gcd operations on polynomials represented as " \
                        "Ruby Arrays of Floats (index = degree convention)."
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
  spec.extensions    = ["ext/polynomial_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
