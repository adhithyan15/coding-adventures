#!/usr/bin/env ruby
# frozen_string_literal: true

# ruby_gem_mirror.rb -- serve a pinned, checksum-verified set of gems.
#
# Why this exists
# ---------------
# Every Ruby package in the repository has its own Gemfile and gemspec, and
# CI runs an unmodified `bundle install` in each one.  Left alone, each of
# those installs resolves the newest allowed version of rake, minitest,
# simplecov and standard from rubygems.org, so a gem release can change a CI
# result with no commit on our side.
#
# A per-package Gemfile.lock would fix that, but frozen locks embed the
# version and dependency list of every path gem, so any gemspec bump would
# invalidate every dependent's lock (292 Gemfiles, ~990 path references).
# Instead there is ONE central lock, code/packages/ruby/third-party/
# Gemfile.lock, and this script makes it the only thing Bundler can see:
#
#   1. `bundle cache --all-platforms --no-install` under BUNDLE_FROZEN=true
#      downloads exactly the locked gems.  Frozen mode fails if the lock is
#      stale relative to the Gemfile, and Bundler verifies every download
#      against the SHA-256 checksums recorded in the lock.
#   2. We build a RubyGems index over those .gem files in a mirror directory.
#   3. We tell Bundler (mirror.https://rubygems.org) to fetch from that
#      directory instead of the network.
#
# From then on any package's `bundle install` can only resolve gems in the
# pinned set: an unlisted gem, or a version outside the locked one, fails
# with "Could not find gem ...".  (Ruby's own default and bundled gems --
# minitest, rake, json ... -- are also candidates because Bundler always sees
# them; Ruby is pinned exactly, so that set is deterministic too.)
#
# The mirror format
# -----------------
# Bundler talks to a mirror like a RubyGems server.  Without the compact
# index endpoints (/versions, /info/<gem>) it falls back to the "full index":
#
#   specs.4.8.gz, latest_specs.4.8.gz, prerelease_specs.4.8.gz
#       gzip(Marshal([[name, Gem::Version, platform], ...]))
#   quick/Marshal.4.8/<name>-<version>[-<platform>].gemspec.rz
#       zlib-deflate(Marshal(Gem::Specification))
#   gems/<name>-<version>[-<platform>].gem
#
# All of that is produced with the standard library; the
# `rubygems-generate_index` gem is deliberately not needed.
#
# Usage
# -----
#   ruby code/scripts/ruby_gem_mirror.rb [--third-party DIR] [--mirror DIR]
#                                        [--global-config] [--github-env]
#
# --global-config runs `bundle config set --global mirror.https://rubygems.org
# file://<mirror>/`, which writes ~/.bundle/config and so reaches every later
# workflow step on every OS (it is what CI uses).  --github-env instead
# appends the equivalent BUNDLE_MIRROR__... variable to $GITHUB_ENV; the
# variable name contains ":" and "/", which POSIX shells cannot export, so
# prefer the config file.  Either way the script then asks Bundler which
# mirror it will use and aborts if it is not ours.

require "fileutils"
require "optparse"
require "rubygems/package"
require "tmpdir"
require "uri"
require "zlib"

REPO_ROOT = File.expand_path("../..", __dir__)
RUBYGEMS_URL = "https://rubygems.org"

# Bundler reads per-source mirrors from environment variables named
# BUNDLE_MIRROR__<URL with ":" -> "__"? ...>.  Rather than re-derive the
# escaping rules we ask Bundler itself (see mirror_env_name).
def mirror_env_name
  # Bundler::Settings#key_for("mirror.https://rubygems.org/") is the
  # authoritative encoding; it is stable across Bundler 2.x-4.x.
  require "bundler"
  Bundler::Settings.new.send(:key_for, "mirror.#{RUBYGEMS_URL}/")
end

def run!(*cmd, env: {}, chdir: nil)
  options = chdir ? {chdir: chdir} : {}
  ok = system(env, *cmd, **options)
  abort("ruby_gem_mirror: command failed: #{cmd.join(" ")}") unless ok
end

# Step 1 -- download the locked gems into a scratch copy of the third-party
# directory (so the checkout never gains a vendor/ or .bundle/ directory).
def cache_locked_gems(third_party, scratch)
  %w[Gemfile Gemfile.lock].each do |name|
    FileUtils.cp(File.join(third_party, name), scratch)
  end
  env = {
    "BUNDLE_FROZEN" => "true",
    "BUNDLE_APP_CONFIG" => File.join(scratch, ".bundle"),
    # Make sure an inherited mirror (e.g. from a previous run) is not used
    # to fetch the gems we are about to verify against the lock.
    mirror_env_name => nil
  }
  run!("bundle", "cache", "--all-platforms", "--no-install", env: env, chdir: scratch)
  Dir[File.join(scratch, "vendor", "cache", "*.gem")].sort
end

def spec_tuple(spec)
  [spec.name, spec.version, spec.original_platform]
end

def write_gz(path, data)
  Zlib::GzipWriter.open(path) { |gz| gz.write(data) }
end

# Step 2 -- build the index.  Release versions go into specs and
# latest_specs; prerelease versions into prerelease_specs.
def build_index(gem_files, mirror)
  FileUtils.rm_rf(mirror)
  %w[gems quick/Marshal.4.8].each { |d| FileUtils.mkdir_p(File.join(mirror, d)) }

  tuples = []
  gem_files.each do |file|
    spec = Gem::Package.new(file).spec
    FileUtils.cp(file, File.join(mirror, "gems", File.basename(file)))
    tuples << spec_tuple(spec)

    # quick/ holds a sanitised, marshalled copy of each gemspec.
    quick = spec.dup
    quick.abbreviate if quick.respond_to?(:abbreviate)
    quick.sanitize if quick.respond_to?(:sanitize)
    File.binwrite(
      File.join(mirror, "quick", "Marshal.4.8", "#{spec.original_name}.gemspec.rz"),
      Zlib::Deflate.deflate(Marshal.dump(quick))
    )
  end

  tuples.sort_by! { |name, version, platform| [name, version, platform.to_s] }
  released, prerelease = tuples.partition { |_, version, _| !version.prerelease? }
  latest = released.group_by { |name, _, platform| [name, platform.to_s] }
    .map { |_, group| group.max_by { |_, version, _| version } }

  write_gz(File.join(mirror, "specs.4.8.gz"), Marshal.dump(released))
  write_gz(File.join(mirror, "latest_specs.4.8.gz"), Marshal.dump(latest))
  write_gz(File.join(mirror, "prerelease_specs.4.8.gz"), Marshal.dump(prerelease))
end

options = {
  third_party: File.join(REPO_ROOT, "code", "packages", "ruby", "third-party"),
  mirror: File.join(ENV.fetch("RUNNER_TEMP", Dir.tmpdir), "ruby-gem-mirror"),
  global_config: false,
  github_env: false
}
OptionParser.new do |o|
  o.on("--third-party DIR", "Directory holding the central Gemfile + lock") { |v| options[:third_party] = v }
  o.on("--mirror DIR", "Directory to build the mirror in") { |v| options[:mirror] = v }
  o.on("--global-config", "Point Bundler at the mirror via `bundle config --global`") { options[:global_config] = true }
  o.on("--github-env", "Export the mirror setting through $GITHUB_ENV") { options[:github_env] = true }
end.parse!

mirror = File.expand_path(options[:mirror])
Dir.mktmpdir("ruby-pins-") do |scratch|
  gems = cache_locked_gems(File.expand_path(options[:third_party]), scratch)
  abort("ruby_gem_mirror: no gems were cached") if gems.empty?
  build_index(gems, mirror)
  puts "ruby_gem_mirror: indexed #{gems.size} pinned gem files in #{mirror}"
end

mirror_url = "file://#{mirror.start_with?("/") ? "" : "/"}#{mirror.tr("\\", "/")}/"
name = mirror_env_name
puts "ruby_gem_mirror: #{name}=#{mirror_url}"
if options[:global_config]
  run!("bundle", "config", "set", "--global", "mirror.#{RUBYGEMS_URL}", mirror_url)
  # Defensive proof for CI logs: the mirror must now be what Bundler reports.
  reported = `bundle config get mirror.#{RUBYGEMS_URL}`
  puts reported
  abort("ruby_gem_mirror: Bundler does not report #{mirror_url} as the mirror") unless reported.include?(mirror_url)
end
if options[:github_env]
  env_file = ENV.fetch("GITHUB_ENV") { abort("ruby_gem_mirror: GITHUB_ENV is not set") }
  File.open(env_file, "a") { |f| f.puts("#{name}=#{mirror_url}") }
end
