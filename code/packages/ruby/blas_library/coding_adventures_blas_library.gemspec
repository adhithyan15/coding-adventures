# frozen_string_literal: true

require_relative "lib/coding_adventures/blas_library/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_blas_library"
  spec.version       = CodingAdventures::BlasLibrary::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Seven-backend BLAS library over simulated GPU hardware"
  spec.description   = "Complete BLAS (Basic Linear Algebra Subprograms) library with " \
                        "CPU, CUDA, Metal, OpenCL, Vulkan, WebGPU, and OpenGL backends. " \
                        "Includes ML extensions (ReLU, GELU, softmax, attention, conv2d). " \
                        "Layer 6 of the accelerator computing stack."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_vendor_api_simulators"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
