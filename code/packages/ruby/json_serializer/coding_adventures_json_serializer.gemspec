# frozen_string_literal: true

require_relative "lib/coding_adventures/json_serializer/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_json_serializer"
  spec.version       = CodingAdventures::JsonSerializer::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Serializes JsonValue objects and native Ruby types to JSON text"
  spec.description   = "Converts JsonValue typed representations or native Ruby types (Hash, Array, etc.) " \
                        "into compact or pretty-printed JSON text with configurable formatting."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_json_value", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
