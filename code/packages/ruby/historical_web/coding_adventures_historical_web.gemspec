# frozen_string_literal: true

require_relative "lib/coding_adventures/historical_web/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_historical_web"
  spec.version       = CodingAdventures::HistoricalWeb::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.email         = ["noreply@example.com"]
  spec.summary       = "CERN 1991 HTTP/0.9 and archived 1992 HTML Tags profiles"
  spec.description   = "In-memory historical Web protocol and markup profiles with typed results."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "required_capabilities.json"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "coding_adventures_test_bootstrap"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "standard", "~> 1.0"
end
