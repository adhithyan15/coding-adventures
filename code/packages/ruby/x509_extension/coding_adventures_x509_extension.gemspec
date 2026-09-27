# frozen_string_literal: true

require_relative "lib/coding_adventures/x509_extension/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_x509_extension"
  spec.version       = CodingAdventures::X509Extension::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Bounded generic X.509 Extension decoding"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_der_asn1", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
end
