# frozen_string_literal: true

# git_diff.rb -- Git-Based Change Detection
# ==========================================
#
# This module uses `git diff` to determine which files changed between
# the current branch and a base ref (typically origin/main). Changed
# files are mapped to packages, and the dependency graph finds everything
# that needs rebuilding.
#
# This is the DEFAULT change detection mode. Git is the source of truth --
# no cache file is needed.
#
# How it works
# ------------
#
# 1. Run `git diff --name-only <base>...HEAD` to get changed file paths.
#    The three-dot diff shows changes since the merge base, which is
#    exactly what we want for PR builds. If three-dot fails (e.g., the
#    base ref doesn't exist), fall back to two-dot diff.
#
# 2. Map each changed file to a package by checking whether the file's
#    path starts with the package's directory path. This is prefix matching:
#    "code/packages/python/foo/src/main.py" starts with
#    "code/packages/python/foo/", so it belongs to package "python/foo".
#
# 3. For Starlark packages with declared srcs, apply STRICT filtering:
#    only trigger a rebuild if the changed file matches one of the
#    declared source patterns (or is a BUILD file). This means editing
#    README.md or CHANGELOG.md in a Starlark package does NOT trigger
#    a rebuild -- only files that actually affect the build matter.
#
# 4. For shell BUILD packages (or Starlark packages without declared
#    srcs), any file change under the package directory triggers a
#    rebuild. This is the legacy behavior.
#
# Strict filtering truth table
# ----------------------------
#
#     Package type     | File changed        | Rebuild?
#     -----------------|---------------------|---------
#     Shell BUILD      | README.md           | YES (any file)
#     Shell BUILD      | src/main.py         | YES (any file)
#     Starlark, no srcs| README.md           | YES (fallback)
#     Starlark, srcs   | README.md           | NO  (not in srcs)
#     Starlark, srcs   | src/main.py         | YES (matches src/**/*.py)
#     Starlark, srcs   | BUILD               | YES (always)
#     Starlark, srcs   | BUILD_mac           | YES (always)
#     Starlark, srcs   | CHANGELOG.md        | NO  (not in srcs)

require "pathname"
require_relative "glob_match"

module BuildTool
  module GitDiff
    module_function

    # get_changed_files -- Run git diff and return changed file paths.
    #
    # Uses three-dot diff (merge base) first, falling back to two-dot
    # diff if that fails. Returns an array of repo-root-relative file
    # paths, or nil if git diff fails entirely.
    #
    # Three-dot diff is preferred because it shows only the changes on
    # the current branch, not changes that happened on the base branch
    # since the branch point. This is what CI needs for PR builds.
    #
    # @param repo_root [String] Path to the repository root.
    # @param diff_base [String] Git ref to diff against (e.g., "origin/main").
    # @return [Array<String>, nil] Changed file paths, or nil on failure.
    def get_changed_files(repo_root, diff_base)
      # Try three-dot diff first (merge base).
      out, status = run_git(repo_root, "diff", "--name-only", "#{diff_base}...HEAD")
      unless status
        # Fallback: two-dot diff.
        out, status = run_git(repo_root, "diff", "--name-only", diff_base, "HEAD")
        return nil unless status
      end

      out.strip.split("\n").map(&:strip).reject(&:empty?)
    end

    # map_files_to_packages -- Map changed file paths to package names.
    #
    # For each changed file, find the package whose directory is a prefix
    # of the file path. Then apply filtering based on the package type:
    #
    #   - Shell BUILD (or Starlark without declared srcs): any file
    #     change triggers a rebuild.
    #   - Starlark with declared srcs: only files matching a declared
    #     source pattern (or BUILD files) trigger a rebuild.
    #
    # This strict filtering is critical for build performance in large
    # monorepos. Without it, editing a README would trigger a full
    # rebuild of the package, wasting CI time on packages that haven't
    # actually changed in any meaningful way.
    #
    # @param changed_files [Array<String>] Repo-root-relative file paths.
    # @param packages [Array<Package>] All discovered packages.
    # @param repo_root [Pathname] Path to the repository root.
    # @return [Hash<String, Boolean>] Package names that need rebuilding.
    def map_files_to_packages(changed_files, packages, repo_root)
      changed = {}
      repo_root = Pathname(repo_root)

      if changed_files.any? { |f| f.start_with?(".github/") || f.start_with?("code/programs/ruby/build-tool/") }
        puts "Git diff: shared files changed -- rebuilding everything"
        packages.each { |p| changed[p.name] = true }
        return changed
      end

      # Build a lookup table of package info with relative paths.
      #
      # We precompute relative paths once rather than computing them
      # inside the inner loop. For a monorepo with 100 packages and
      # 50 changed files, this avoids 5000 Pathname#relative_path_from
      # calls.
      pkg_infos = packages.filter_map do |pkg|
        rel = begin
          pkg.path.relative_path_from(repo_root).to_s
        rescue ArgumentError
          next nil
        end
        # Normalize to forward slashes for consistent matching.
        rel = rel.tr("\\", "/")

        {
          name: pkg.name,
          rel_path: rel,
          is_starlark: pkg.respond_to?(:is_starlark) ? pkg.is_starlark : false,
          declared_srcs: pkg.respond_to?(:declared_srcs) ? (pkg.declared_srcs || []) : []
        }
      end

      changed_files.each do |file|
        # Normalize to forward slashes.
        file = file.tr("\\", "/")

        pkg_infos.each do |info|
          # Check if the file is under this package's directory.
          # The file must either equal the package path or start with
          # "package_path/" to avoid false matches (e.g., "foo" matching
          # "foobar/baz.py").
          unless file.start_with?("#{info[:rel_path]}/") || file == info[:rel_path]
            next
          end

          # File is under this package's directory.
          if !info[:is_starlark] || info[:declared_srcs].empty?
            # Shell BUILD or Starlark without declared srcs:
            # any file change triggers a rebuild.
            changed[info[:name]] = true
            break
          end

          # Starlark package with declared srcs: strict filtering.
          # Get the file path relative to the package directory.
          rel_to_package = file.delete_prefix("#{info[:rel_path]}/")

          # BUILD file changes always trigger a rebuild -- the build
          # definition itself changed.
          basename = File.basename(rel_to_package)
          if basename == "BUILD" || basename.start_with?("BUILD_")
            changed[info[:name]] = true
            break
          end

          # Check if the file matches any declared source pattern.
          info[:declared_srcs].each do |pattern|
            if GlobMatch.match_path?(pattern, rel_to_package)
              changed[info[:name]] = true
              break
            end
          end

          # File matched to this package -- don't check other packages.
          break
        end
      end

      changed
    end

    # -- Private helpers -------------------------------------------------------

    # run_git -- Execute a git command and return [output, success].
    #
    # @param repo_root [String] Working directory for the git command.
    # @param *args [Array<String>] Git subcommand and arguments.
    # @return [Array(String, Boolean)] Output string and success flag.
    def run_git(repo_root, *args)
      require "open3"
      stdout, _stderr, status = Open3.capture3("git", *args, chdir: repo_root.to_s)
      [stdout, status.success?]
    rescue StandardError
      ["", false]
    end
  end
end
