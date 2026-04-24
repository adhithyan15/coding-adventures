# frozen_string_literal: true

require_relative "lib/qr_code/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_qr_code"
  spec.version = QrCode::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "QR Code encoder — ISO/IEC 18004:2015 compliant, all ECC levels, versions 1-40"
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_barcode_2d", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
end
