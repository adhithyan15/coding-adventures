# frozen_string_literal: true

require_relative "lib/coding_adventures/virtual_memory/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_virtual_memory"
  spec.version       = CodingAdventures::VirtualMemory::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "D13 Virtual Memory -- page tables, TLB, frame allocator, page replacement, and MMU"
  spec.description   = "A complete virtual memory subsystem implementing page tables (single and two-level Sv32), " \
                        "TLB with LRU eviction, physical frame allocator with bitmap, page replacement policies " \
                        "(FIFO, LRU, Clock), and a full MMU with copy-on-write fork support."
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
