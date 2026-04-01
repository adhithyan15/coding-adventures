Gem::Specification.new do |s|
  s.name        = "font-parser-native"
  s.version     = "0.1.0"
  s.summary     = "Rust-backed OpenType/TrueType font parser — Ruby native extension"
  s.description = "Ruby C extension wrapping the Rust font-parser core. Zero external dependencies."
  s.authors     = ["Coding Adventures"]
  s.license     = "MIT"
  s.files       = Dir["lib/**/*", "src/**/*", "Cargo.toml", "README.md"]
  s.extensions  = ["Cargo.toml"]  # rb_sys / thermite convention
  s.required_ruby_version = ">= 2.7"
end
