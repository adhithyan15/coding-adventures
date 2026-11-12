---
category: Ruby
---

# Per-package frozen Gemfile.locks break whenever a path gem's gemspec changes, so the repo pins third-party gems with one central lock served through a local mirror

Ruby packages had no pinning at all: each of the ~290 Gemfiles resolved the
newest rake/minitest/simplecov/standard from rubygems.org on every CI run, so
a gem release could change a result with no commit on our side.

The obvious fix, a committed `Gemfile.lock` per package, does not work. A lock
embeds the version and dependency list of every `path:` gem, and frozen mode
rejects it ("gemspecs for path gems changed") when any of them changes. With
about 990 path references, one gemspec bump would invalidate every dependent's
lock.

What works: ONE central lock, `code/packages/ruby/third-party/Gemfile.lock`
(Gemfile lists the allowlisted gems with the union of all ranges; generated
with `bundle lock --add-checksums` and the CI platforms). CI runs
`code/scripts/ruby_gem_mirror.rb`: `BUNDLE_FROZEN=true bundle cache
--all-platforms --no-install` verifies the lock against its Gemfile and the
checksums, a stdlib-only full index (`specs.4.8.gz`, `quick/Marshal.4.8`,
`gems/`) is built over the cached `.gem` files, and
`bundle config set --global mirror.https://rubygems.org file://...` points
Bundler at it. Every package's unchanged `bundle install` can then only see
the pinned set; an unlisted gem or out-of-range version fails.
`code/scripts/ruby_pins_check.py` (contracts job) fails when a manifest uses a
gem or range the central Gemfile/lock does not cover.

Do not add per-package Gemfile.lock files, and when a manifest needs a new
gem or range, edit the central Gemfile and re-run `bundle lock
--add-checksums` there. Ruby's default/bundled gems still count as candidates;
that is deterministic because Ruby is pinned exactly.
