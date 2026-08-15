# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_chief_of_staff_channel_crypto"
  spec.version = "0.1.0"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Portable D18F messages and D18Q key grants for Chief of Staff channels"
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => spec.homepage,
    "rubygems_mfa_required" => "true"
  }
  spec.add_dependency "base64", ">= 0.2.0"
  spec.add_dependency "json", ">= 2.21.0"
  spec.add_dependency "coding_adventures_chacha20_poly1305", ">= 0.1.0"
  spec.add_dependency "coding_adventures_ed25519", ">= 0.1.0"
  spec.add_dependency "coding_adventures_hkdf", ">= 0.1.0"
  spec.add_dependency "coding_adventures_sha256", ">= 0.1.0"
  spec.add_dependency "coding_adventures_x25519", ">= 0.1.0"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
