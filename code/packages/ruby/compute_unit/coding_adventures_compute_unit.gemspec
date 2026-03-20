# frozen_string_literal: true

require_relative "lib/coding_adventures/compute_unit/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_compute_unit"
  spec.version       = CodingAdventures::ComputeUnit::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Compute unit simulators for GPU, TPU, and NPU architectures"
  spec.description   = "Implements five compute unit architectures (NVIDIA SM, AMD CU, Google TPU MXU, " \
                        "Intel Xe Core, Apple ANE Core) that manage parallel execution engines, " \
                        "schedulers, shared memory, and caches. Layer 7 of the accelerator stack."
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

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
