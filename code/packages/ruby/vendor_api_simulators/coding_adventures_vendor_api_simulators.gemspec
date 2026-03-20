# frozen_string_literal: true

require_relative "lib/coding_adventures/vendor_api_simulators/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_vendor_api_simulators"
  spec.version       = CodingAdventures::VendorApiSimulators::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Six vendor GPU API simulators over one compute runtime"
  spec.description   = "CUDA, OpenCL, Metal, Vulkan, WebGPU, and OpenGL simulators " \
                        "built on the Vulkan-inspired compute runtime. " \
                        "Layer 3 of the accelerator computing stack."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_compute_runtime"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
