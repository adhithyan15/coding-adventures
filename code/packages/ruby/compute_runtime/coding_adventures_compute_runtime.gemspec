# frozen_string_literal: true

require_relative "lib/coding_adventures/compute_runtime/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_compute_runtime"
  spec.version       = CodingAdventures::ComputeRuntime::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Vulkan-inspired compute runtime for accelerator devices"
  spec.description   = "A low-level compute runtime that provides device discovery, " \
                        "command buffers, memory management, pipelines, and synchronization " \
                        "primitives. Layer 5 of the accelerator computing stack."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_device_simulator"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
