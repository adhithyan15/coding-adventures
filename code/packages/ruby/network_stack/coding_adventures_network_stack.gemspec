# frozen_string_literal: true

require_relative "lib/coding_adventures/network_stack/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_network_stack"
  spec.version       = CodingAdventures::NetworkStack::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Full layered networking stack: Ethernet, IP, TCP, UDP, sockets, DNS, HTTP"
  spec.description   = "Implements a complete network stack from Layer 2 (Ethernet frames) through Layer 7 (HTTP), including ARP resolution, IP routing, TCP state machine with 3-way handshake, UDP datagrams, Berkeley sockets API, DNS resolution, and HTTP client/server — all connected via a simulated NetworkWire."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
