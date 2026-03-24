# frozen_string_literal: true

require_relative "lib/coding_adventures/json_value/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_json_value"
  spec.version       = CodingAdventures::JsonValue::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Typed JSON value representation with AST-to-value and native type conversion"
  spec.description   = "Converts json-parser ASTs into typed JsonValue objects (Object, Array, String, " \
                        "Number, Boolean, Null) and provides bidirectional conversion to native Ruby types. " \
                        "The bridge between parsing and application-level JSON usage."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_json_parser", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
