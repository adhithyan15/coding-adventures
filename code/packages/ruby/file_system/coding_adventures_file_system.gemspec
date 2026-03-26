# frozen_string_literal: true

require_relative "lib/coding_adventures/file_system/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_file_system"
  spec.version       = CodingAdventures::FileSystem::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "D15 File System -- inode-based VFS with directories, file descriptors, and block I/O"
  spec.description   = "A simplified ext2-inspired file system implementing inodes, directories, " \
                        "block bitmaps, path resolution, file descriptors (with dup/dup2), and a " \
                        "complete VFS layer with open/read/write/close/mkdir/unlink operations."
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
