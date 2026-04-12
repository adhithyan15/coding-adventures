# frozen_string_literal: true

require_relative "lib/coding_adventures/irc_server/version"

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_irc_server"
  spec.version = CodingAdventures::IrcServer::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.email   = ["adhithyan15@users.noreply.github.com"]

  spec.summary     = "RFC 1459 IRC server state machine"
  spec.description = "Level 2 of the coding-adventures IRC stack. Pure state machine for IRC protocol handling — channels, nicks, command dispatch. Zero I/O."
  spec.homepage    = "https://github.com/coding-adventures/irc_server"
  spec.license     = "MIT"

  spec.required_ruby_version = ">= 3.0"

  spec.files         = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_irc_proto", "~> 0.1"
end
