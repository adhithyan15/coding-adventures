# ============================================================================
# ruby_library.star — Build rule for Ruby library packages
# ============================================================================
#
# Ruby packages in this monorepo are structured as gems — each has a .gemspec
# file, a Gemfile, and follows the standard Ruby gem layout:
#
#   my-package/
#     lib/
#       coding_adventures_my_package.rb    # entry point
#       coding_adventures_my_package/
#         implementation.rb                # actual code
#     test/
#       test_my_package.rb                 # tests
#     Gemfile                              # dependencies
#     Rakefile                             # task definitions
#     coding_adventures_my_package.gemspec  # gem metadata
#
# RUBY DEPENDENCY MANAGEMENT
# --------------------------
# Ruby uses Bundler to manage dependencies. For monorepo packages that depend
# on sibling packages, the Gemfile uses path references:
#
#   gem "coding_adventures_transistors", path: "../transistors"
#
# This is analogous to Go's replace directives or TypeScript's file: deps.
# The ruby_library rule's deps field must mirror these Gemfile path references
# so the build tool knows the dependency graph.
#
# IMPORTANT LESSONS LEARNED (see lessons.md):
#   - Gemfiles must list ALL transitive path dependencies, not just direct ones
#   - require ordering matters: require dependencies before own modules
#   - Ruby predicate methods use ? suffix (contains? not contains)
#   - BUILD files use bare commands (no mise prefix) for CI compatibility
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:ruby_library.star", "ruby_library")
#
#   ruby_library(
#       name = "logic-gates",
#       srcs = ["lib/**/*.rb", "test/**/*.rb"],
#       deps = ["ruby/transistors"],
#       test_runner = "minitest",
#   )
#
# ============================================================================

def ruby_library(name, srcs = [], deps = [], test_runner = "minitest"):
    # Register a Ruby library target for the build system.
    #
    # Ruby libraries are built as gems with Bundler managing dependencies.
    # The build tool will run:
    #     bundle install --quiet    — install gem dependencies
    #     bundle exec rake test     — run the test suite via Rake
    #
    # Args:
    #     name: The package name, matching the directory under
    #           code/packages/ruby/. For example, "logic-gates" corresponds
    #           to code/packages/ruby/logic-gates/.
    #
    #           Note: Ruby gem names use underscores (coding_adventures_logic_gates)
    #           but the directory and build target use hyphens (logic-gates).
    #
    #     srcs: File paths or glob patterns for change detection.
    #           Typical patterns for Ruby:
    #               ["lib/**/*.rb"]                    — library source only
    #               ["lib/**/*.rb", "test/**/*.rb"]    — source and tests
    #               ["lib/**/*.rb", "*.gemspec"]       — source and gem metadata
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           These must match the path: references in the Gemfile.
    #           Examples:
    #               ["ruby/transistors"]
    #               ["ruby/logic-gates", "ruby/arithmetic"]
    #
    #           IMPORTANT: Include transitive dependencies too! If your package
    #           depends on A, and A depends on B, list BOTH A and B in deps.
    #           This matches the Gemfile, which must also list both.
    #
    #     test_runner: Which test framework to use. Currently supported:
    #
    #           "minitest" — (default) Ruby's built-in test framework.
    #                        Lightweight, fast, and included in Ruby stdlib.
    #                        Runs via: bundle exec rake test
    #
    #           "rspec"    — Behavior-driven development framework.
    #                        More expressive syntax (describe/it blocks).
    #                        Runs via: bundle exec rspec
    #
    #           Most packages in this monorepo use minitest for simplicity.
    return {
        # "ruby_library" triggers Ruby-specific build logic:
        #   - bundle install to resolve gem dependencies
        #   - standardrb for linting (required by repo standards)
        #   - minitest or rspec for testing
        "rule": "ruby_library",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "test_runner": test_runner,
        "commands": [
            {"type": "cmd", "program": "bundle", "args": ["install", "--quiet"]},
            {"type": "cmd", "program": "bundle", "args": ["exec", "rake", "test"]},
        ],
    }
