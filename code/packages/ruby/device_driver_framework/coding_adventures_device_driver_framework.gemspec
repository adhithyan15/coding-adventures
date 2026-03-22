# frozen_string_literal: true

require_relative "lib/coding_adventures/device_driver_framework/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_device_driver_framework"
  spec.version = CodingAdventures::DeviceDriverFramework::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "D12 Device Driver Framework — unified device abstraction for character, block, and network devices"
  spec.description = "Implements a unified device driver abstraction with three device families " \
    "(CharacterDevice, BlockDevice, NetworkDevice), a DeviceRegistry for registration and " \
    "lookup, and concrete simulated implementations (disk, keyboard, display, NIC)."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
