# frozen_string_literal: true

# Copy the checked neutral registry into the installed Ruby package. Keeping
# this a byte-for-byte copy makes review and stale-snapshot detection simple.
require "pathname"

package_root = Pathname(__dir__).parent
repository_root = package_root.parent.parent.parent.parent
checked = repository_root / "code/specs/fixtures/build-tool-v1/language-source-input-registry.json"
packaged = package_root / "lib/build_tool/language_source_input_registry.json"
bytes = checked.binread
if ARGV == ["--check"]
  abort "Ruby source-input registry snapshot is stale" unless packaged.file? && packaged.binread == bytes
elsif ARGV.empty?
  packaged.binwrite(bytes)
else
  abort "usage: ruby tools/sync_language_source_input_registry.rb [--check]"
end
