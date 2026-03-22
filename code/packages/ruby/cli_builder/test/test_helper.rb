# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
end

require "minitest/autorun"
require "coding_adventures_cli_builder"
require "json"
require "tempfile"

# Convenience helper: write a spec hash to a tempfile and yield its path.
# The tempfile is deleted after the block.
def with_spec_file(spec_hash)
  f = Tempfile.new(["cli_spec", ".json"])
  f.write(JSON.generate(spec_hash))
  f.close
  yield f.path
ensure
  f.unlink
end

# Parse argv against an inline spec hash (no file I/O needed).
def parse_with_spec(spec_hash, argv)
  CodingAdventures::CliBuilder::Parser.new(nil, argv, spec_hash: spec_hash).parse
end
