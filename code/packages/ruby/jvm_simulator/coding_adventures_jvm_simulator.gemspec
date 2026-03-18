# frozen_string_literal: true

require_relative "lib/coding_adventures/jvm_simulator/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_jvm_simulator"
  spec.version = CodingAdventures::JvmSimulator::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "JVM bytecode simulator with real opcode values"
  spec.description = "Simulates JVM bytecode: iconst, bipush, ldc, iload/istore, " \
    "iadd/isub/imul/idiv, goto, if_icmpeq, if_icmpgt, ireturn, return."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
