# frozen_string_literal: true

require_relative "lib/coding_adventures/rpc/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_rpc"
  spec.version       = CodingAdventures::Rpc::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Codec-agnostic RPC primitive for building protocol-specific packages"
  spec.description   = "Abstract Remote Procedure Call layer that separates method dispatch, " \
                        "id correlation, and error handling from serialisation (codec) and " \
                        "stream framing. json-rpc and future codec-specific packages build on top."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # No runtime dependencies — stdlib only (stringio)
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
