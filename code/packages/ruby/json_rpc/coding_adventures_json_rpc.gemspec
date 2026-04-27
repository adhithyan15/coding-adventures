# frozen_string_literal: true

require_relative "lib/coding_adventures/json_rpc/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_json_rpc"
  spec.version       = CodingAdventures::JsonRpc::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "JSON-RPC 2.0 over stdin/stdout with Content-Length framing"
  spec.description   = "Implements the JSON-RPC 2.0 transport layer used by Language Server Protocol " \
                        "servers. Provides MessageReader, MessageWriter, and Server with method dispatch."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.2.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # No runtime dependencies — stdlib only (json, stringio)
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
