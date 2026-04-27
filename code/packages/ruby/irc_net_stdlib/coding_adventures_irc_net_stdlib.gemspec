# frozen_string_literal: true

require_relative "lib/coding_adventures/irc_net_stdlib/version"

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_irc_net_stdlib"
  spec.version = CodingAdventures::IrcNetStdlib::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.email   = ["adhithyan15@users.noreply.github.com"]

  spec.summary     = "Ruby stdlib TCP event loop for IRC"
  spec.description = "Level 3 of the coding-adventures IRC stack. Thread-per-connection TCP event loop using Ruby's standard library TCPServer."
  spec.homepage    = "https://github.com/coding-adventures/irc_net_stdlib"
  spec.license     = "MIT"

  spec.required_ruby_version = ">= 3.0"

  spec.files         = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]
end
