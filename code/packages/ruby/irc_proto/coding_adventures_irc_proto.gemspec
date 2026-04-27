# frozen_string_literal: true

require_relative "lib/coding_adventures/irc_proto/version"

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_irc_proto"
  spec.version = CodingAdventures::IrcProto::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.email   = ["adhithyan15@users.noreply.github.com"]

  spec.summary     = "Pure IRC message parsing and serialisation"
  spec.description = "Level 0 of the coding-adventures IRC stack. Parses IRC wire-format lines into Message objects and serialises them back. Zero I/O."
  spec.homepage    = "https://github.com/coding-adventures/irc_proto"
  spec.license     = "MIT"

  spec.required_ruby_version = ">= 3.0"

  spec.files = Dir["lib/**/*.rb"]

  spec.require_paths = ["lib"]
end
