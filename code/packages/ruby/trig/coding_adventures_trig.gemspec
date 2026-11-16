# frozen_string_literal: true

require_relative "lib/trig/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_trig"
  spec.version       = Trig::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Sine, cosine, tangent, arctangent and square root from first principles (TRIG-REF-1)"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }
end
