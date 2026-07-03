# frozen_string_literal: true

require_relative "lib/coding_adventures/irc_server_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_irc_server_native"
  spec.version       = CodingAdventures::IrcServerNative::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "High-performance IRC server for Ruby, backed by the all-Rust irc-net-reactor engine"
  spec.description   = "A native extension that embeds the irc-net-reactor Rust IRC engine (on the " \
                       "home-grown kqueue/epoll reactor) via ruby-bridge. Ruby launches and controls " \
                       "the server; all IRC and TCP logic runs in Rust."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"

  spec.files         = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "README.md",
    "CHANGELOG.md"
  ]
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/irc_server_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
