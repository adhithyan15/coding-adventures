# frozen_string_literal: true

require_relative "lib/coding_adventures/html_renderer/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_html_renderer"
  spec.version = CodingAdventures::HtmlRenderer::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "HTML renderer for pipeline visualization (shell gem)"
  spec.description = "Shell gem for the HTML renderer package. Implementation forthcoming."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
