# frozen_string_literal: true

require "json"
require_relative "../lib/unix_history"

def assert_equal(expected, actual)
  return if expected == actual

  raise "Expected #{expected.inspect}, got #{actual.inspect}"
end

fixtures = File.expand_path("../../../../specs/fixtures/historical-unix-v1/file-read.json", __dir__)
JSON.parse(File.read(fixtures, encoding: "UTF-8")).each do |fixture|
  expected = fixture.slice("output", "events")
  actual = JSON.parse(JSON.generate(UnixHistory.run_command(fixture.fetch("command"))))
  assert_equal(expected, actual)
end

fs = UnixHistory::FileSystem.new
assert_equal(5, fs.resolve("//etc/../etc/./notes").number)
assert_equal("ENOTDIR", fs.resolve("/hello/.."))
assert_equal("EINVAL", fs.resolve("a\0b"))

table = UnixHistory::DescriptorTable.new(2)
assert_equal(0, table.open(2))
assert_equal(1, table.open(2))
assert_equal(nil, table.open(2))
table.advance(0, 4)
assert_equal(0, table.get(1).offset)
assert_equal(false, table.close(-1))
assert_equal(true, table.close(0))
assert_equal(0, table.open(3))
assert_equal(0, table.get(0).offset)

kernel = UnixHistory::Kernel.new
assert_equal("EISDIR", kernel.open("/")[:error])
assert_equal("EBADF", kernel.read(0, 8).first[:error])
kernel.open("/hello")
assert_equal("EINVAL", kernel.read(0, -1).first[:error])
assert_equal("He", kernel.read(0, 2).last)
assert_equal([], UnixHistory.run_command("cat")[:events])
puts "Ruby fixture replay and component checks passed"
