# frozen_string_literal: true

require_relative "lib/coding_adventures/irc_framing/version"

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_irc_framing"
  spec.version = CodingAdventures::IrcFraming::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.email   = ["adhithyan15@users.noreply.github.com"]

  spec.summary     = "IRC TCP byte-stream to line-frame converter"
  spec.description = "Level 1 of the coding-adventures IRC stack. Stateful framer that buffers raw TCP bytes and yields complete IRC lines."
  spec.homepage    = "https://github.com/coding-adventures/irc_framing"
  spec.license     = "MIT"

  spec.required_ruby_version = ">= 3.0"

  spec.files         = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_irc_proto", "~> 0.1"
end
