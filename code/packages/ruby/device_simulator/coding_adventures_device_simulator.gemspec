# frozen_string_literal: true

require_relative "lib/coding_adventures/device_simulator/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_device_simulator"
  spec.version       = CodingAdventures::DeviceSimulator::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Complete accelerator device simulators for GPU, TPU, and NPU architectures"
  spec.description   = "Implements five device simulators (NVIDIA GPU, AMD GPU, Google TPU, " \
                        "Intel GPU, Apple ANE) that assemble compute units with global memory, " \
                        "L2 cache, and work distributors into full devices. Layer 6 of the accelerator stack."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_gpu_core"
  spec.add_dependency "coding_adventures_fp_arithmetic"
  spec.add_dependency "coding_adventures_parallel_execution_engine"
  spec.add_dependency "coding_adventures_clock"
  spec.add_dependency "coding_adventures_compute_unit"
  spec.add_dependency "coding_adventures_cache"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
