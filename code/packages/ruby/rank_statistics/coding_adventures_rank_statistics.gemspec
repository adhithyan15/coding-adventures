# frozen_string_literal: true

require_relative "lib/coding_adventures/rank_statistics/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_rank_statistics"
  spec.version = CodingAdventures::RankStatistics::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Reusable finite-sample median and exact median confidence interval"
  spec.description = "Finite binary64 sample medians and exact order-statistic confidence intervals " \
                     "using Ruby's arbitrary-precision integer arithmetic."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb", "testdata/*.json", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "coding_adventures_test_bootstrap", "~> 0.2.2"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "standard", "~> 1.0"
end
