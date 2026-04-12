# frozen_string_literal: true

require_relative "lib/coding_adventures/ls00/version"

Gem::Specification.new do |s|
  s.name        = "coding_adventures_ls00"
  s.version     = CodingAdventures::Ls00::VERSION
  s.summary     = "Generic Language Server Protocol (LSP) framework"
  s.description = "A generic LSP server framework that language-specific 'bridges' plug into " \
                   "using Ruby's duck typing. Handles all protocol boilerplate: JSON-RPC transport, " \
                   "document synchronization, capability negotiation, and semantic token encoding."
  s.authors     = ["Coding Adventures"]
  s.homepage    = "https://github.com/adhithyan15/coding-adventures"
  s.license     = "MIT"
  s.required_ruby_version = ">= 3.2.0"
  s.files       = Dir["lib/**/*.rb"]
  s.require_paths = ["lib"]
  s.metadata    = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  s.add_dependency "coding_adventures_json_rpc"
  s.add_development_dependency "minitest", "~> 5.0"
  s.add_development_dependency "rake", "~> 13.0"
end
