# frozen_string_literal: true

require_relative "lib/coding_adventures/ls00/version"

Gem::Specification.new do |spec|
  spec.name        = "coding_adventures_ls00"
  spec.version     = CodingAdventures::Ls00::VERSION
  spec.summary     = "Generic Language Server Protocol (LSP) framework"
  spec.description = "A generic LSP server framework that language-specific 'bridges' plug into " \
                      "using Ruby's duck typing. Handles all protocol boilerplate: JSON-RPC transport, " \
                      "document synchronization, capability negotiation, and semantic token encoding."
  spec.authors     = ["Coding Adventures"]
  spec.homepage    = "https://github.com/adhithyan15/coding-adventures"
  spec.license     = "MIT"
  spec.required_ruby_version = ">= 3.2.0"
  spec.files       = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]
  spec.metadata    = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "coding_adventures_json_rpc"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
