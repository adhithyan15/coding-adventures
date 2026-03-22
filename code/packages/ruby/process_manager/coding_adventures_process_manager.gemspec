# frozen_string_literal: true

require_relative "lib/coding_adventures/process_manager/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_process_manager"
  spec.version       = CodingAdventures::ProcessManager::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "D14 Process Manager -- fork, exec, wait, signals, priority scheduling"
  spec.description   = "A complete process management subsystem implementing Unix-style fork/exec/wait, " \
                        "POSIX signal delivery and handling, process control blocks with lifecycle management, " \
                        "and priority-based scheduling with round-robin within priority levels."
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
