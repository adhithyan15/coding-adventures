# frozen_string_literal: true

require_relative "lib/coding_adventures/os_kernel/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_os_kernel"
  spec.version       = CodingAdventures::OsKernel::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "S04 OS Kernel -- minimal monolithic kernel with process management, scheduler, and syscalls"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = { "source_code_uri" => "https://github.com/adhithyan15/coding-adventures", "rubygems_mfa_required" => "true" }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
