# frozen_string_literal: true

# Copy the checked neutral registry into the installed Ruby package. Keeping
# this a byte-for-byte copy makes review and stale-snapshot detection simple.
require "pathname"
require "tempfile"

package_root = Pathname(__dir__).parent
repository_root = package_root.parent.parent.parent.parent
checked = repository_root / "code/specs/fixtures/build-tool-v1/language-source-input-registry.json"
packaged = package_root / "lib/build_tool/language_source_input_registry.json"

# Do not let a checkout-level link turn a maintenance command into a write
# outside this package. Rename replaces the destination directory entry
# instead of opening a possibly substituted destination for writing.
def require_regular_path!(path, directory: false)
  status = path.lstat
  valid = directory ? status.directory? : status.file?
  abort "registry sync path is not a regular #{directory ? "directory" : "file"}: #{path}" unless valid && !status.symlink?
end

[repository_root, repository_root / "code", repository_root / "code/specs",
  repository_root / "code/specs/fixtures",
  repository_root / "code/specs/fixtures/build-tool-v1",
  package_root, package_root / "tools", package_root / "lib", packaged.dirname].each do |dir|
  require_regular_path!(dir, directory: true)
end
require_regular_path!(checked)
require_regular_path!(packaged) if packaged.exist? || packaged.symlink?
bytes = checked.binread
if ARGV == ["--check"]
  abort "Ruby source-input registry snapshot is stale" unless packaged.file? && packaged.binread == bytes
elsif ARGV.empty?
  Tempfile.create([".language-source-input-registry-", ".json"], packaged.dirname) do |temporary|
    temporary.binmode
    temporary.write(bytes)
    temporary.flush
    temporary.close
    require_regular_path!(packaged.dirname, directory: true)
    require_regular_path!(packaged) if packaged.exist? || packaged.symlink?
    File.rename(temporary.path, packaged)
  end
else
  abort "usage: ruby tools/sync_language_source_input_registry.rb [--check]"
end
