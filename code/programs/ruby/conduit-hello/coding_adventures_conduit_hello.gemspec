# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_conduit_hello"
  spec.version = "0.1.0"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.email   = ["adhithyan15@users.noreply.github.com"]

  spec.summary     = "Hello-world demo for the Conduit web framework"
  spec.description = "A minimal demo program that exercises Conduit's Sinatra-style routing DSL. Serves a root greeting and a personalised /hello/:name route via the native Rust HTTP engine."
  spec.homepage    = "https://github.com/adhithyan15/coding-adventures"
  spec.license     = "MIT"

  spec.required_ruby_version = ">= 3.0"

  spec.files = Dir["lib/**/*.rb", "*.rb"]

  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_conduit"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake",     "~> 13.0"

  spec.metadata = {
    "source_code_uri"       => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
end
