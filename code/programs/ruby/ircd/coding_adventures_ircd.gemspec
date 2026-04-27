# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_ircd"
  spec.version = "0.1.0"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.email   = ["adhithyan15@users.noreply.github.com"]

  spec.summary     = "IRC server daemon — wires irc_net_stdlib, irc_framing, irc_server, and irc_proto"
  spec.description = "Top-level IRC server program for the coding-adventures IRC stack. Connects the network I/O layer (irc_net_stdlib) to the framing layer (irc_framing), protocol layer (irc_proto), and state-machine layer (irc_server)."
  spec.homepage    = "https://github.com/adhithyan15/coding-adventures"
  spec.license     = "MIT"

  spec.required_ruby_version = ">= 3.0"

  spec.files = Dir["lib/**/*.rb", "bin/**/*"]

  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_irc_proto"
  spec.add_dependency "coding_adventures_irc_framing"
  spec.add_dependency "coding_adventures_irc_server"
  spec.add_dependency "coding_adventures_irc_net_stdlib"

  spec.add_development_dependency "minitest",  "~> 5.0"
  spec.add_development_dependency "simplecov", ">= 0"
  spec.add_development_dependency "rake",      "~> 13.0"

  spec.metadata = {
    "source_code_uri"      => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
end
