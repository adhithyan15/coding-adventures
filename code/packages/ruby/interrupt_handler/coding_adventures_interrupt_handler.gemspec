# frozen_string_literal: true

require_relative "lib/coding_adventures/interrupt_handler/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_interrupt_handler"
  spec.version = CodingAdventures::InterruptHandler::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "S03 Interrupt Handler — IDT, ISR registry, controller, context save/restore"
  spec.description = "Implements the full interrupt lifecycle for the coding-adventures " \
    "simulated computer: Interrupt Descriptor Table, ISR registry, interrupt controller " \
    "with pending queue/masking/priority, and CPU context save/restore."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
