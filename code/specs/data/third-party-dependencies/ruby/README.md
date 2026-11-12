# third-party (central gem lock)

Not a package. This directory pins every third-party gem the repository's Ruby
packages may use.

- `Gemfile` lists the allowlisted gems
  (`code/specs/data/third-party-dependencies/ruby-allowlist.json`), each with
  the union of the requirement ranges used by any Gemfile or gemspec.
- `Gemfile.lock` is committed, with checksums, for the platforms CI uses
  (Linux, Windows, macOS arm64 and x86_64). It is the only `Gemfile.lock` in
  the repository that is tracked.

CI runs `ruby code/scripts/ruby_gem_mirror.rb --global-config` before any
Ruby BUILD. It downloads exactly the locked gems (`bundle cache` under
`BUNDLE_FROZEN`, which verifies checksums and fails on a stale lock), builds
a local RubyGems index over them and points Bundler's
`mirror.https://rubygems.org` at it, so every package's unchanged
`bundle install` resolves only from the pinned set. Ruby's own default and
bundled gems stay candidates; Ruby is pinned exactly, so that is
deterministic too. `code/scripts/ruby_pins_check.py` keeps the Gemfile and
lock in sync with all manifests.

To change the pins, edit `Gemfile` and run, with Bundler 4.0.24:

    bundle lock --add-checksums
    for p in x86_64-linux x64-mingw-ucrt arm64-darwin x86_64-darwin; do
      bundle lock --add-platform $p; done
