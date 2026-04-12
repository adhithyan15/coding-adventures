# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_b_plus_tree"
  spec.version       = "0.1.0"
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "A B+ tree with leaf linked list for efficient range scans (DT12)"
  spec.description   = "A full-featured, in-memory B+ tree. Values stored only in leaves. " \
                        "Leaf nodes linked in a sorted linked list for O(log n + k) range scans " \
                        "and O(n) full scans without visiting internal nodes. Includes Enumerable."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
