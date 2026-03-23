# frozen_string_literal: true

require_relative "lib/coding_adventures/uuid/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_uuid"
  spec.version       = CodingAdventures::Uuid::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "UUID v1/v3/v4/v5/v7 generation and parsing (RFC 4122 + RFC 9562)"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.add_dependency "coding_adventures_sha1", "~> 0.1"
  spec.add_dependency "coding_adventures_md5", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
end
