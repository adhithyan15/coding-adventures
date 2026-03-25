# ============================================================================
# ruby_binary.star — Build rule for Ruby executable programs
# ============================================================================
#
# Ruby programs are scripts that run via the Ruby interpreter. Unlike Go or
# Rust, Ruby doesn't compile to a standalone binary — you always need the
# Ruby runtime installed. A Ruby "binary" in build system terms is simply
# a Ruby script with an entry point that's meant to be executed directly.
#
# Ruby programs can be distributed as:
#   1. A script: ruby main.rb
#   2. A gem with an executable: gem install my-tool && my-tool
#   3. A Bundler binstub: bundle exec my-tool
#
# In this monorepo, Ruby programs live under code/programs/ruby/<name>/ and
# are executed via their entry point script.
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:ruby_binary.star", "ruby_binary")
#
#   ruby_binary(
#       name = "code-formatter",
#       srcs = ["lib/**/*.rb", "*.rb"],
#       deps = ["ruby/parser", "ruby/cli-builder"],
#       entry_point = "main.rb",
#   )
#
# ============================================================================

def ruby_binary(name, srcs = [], deps = [], entry_point = "main.rb"):
    # Register a Ruby binary (executable program) target.
    #
    # Ruby binaries are scripts that run via the Ruby interpreter. The build
    # tool will:
    #     bundle install --quiet       — install gem dependencies
    #     bundle exec rake test        — run tests if they exist
    #     bundle exec ruby <entry_point> --help — smoke test
    #
    # Args:
    #     name: The program name, matching the directory under
    #           code/programs/ruby/. For example, "code-formatter" maps to
    #           code/programs/ruby/code-formatter/.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           Typical: ["lib/**/*.rb", "*.rb", "Gemfile"]
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           Examples:
    #               ["ruby/parser"]
    #               ["ruby/lexer", "ruby/cli-builder"]
    #
    #           Remember: include transitive dependencies too (same rule as
    #           ruby_library).
    #
    #     entry_point: The Ruby file to execute when running this program.
    #           Defaults to "main.rb".
    #
    #           Examples:
    #               "main.rb"          — simple script in package root
    #               "bin/my-tool"      — executable in bin/ directory
    #               "lib/cli.rb"       — CLI entry point in lib/
    #
    #           Ruby conventions vary more than Go (which always uses main()).
    #           The entry_point parameter lets each program specify its own
    #           convention.
    return {
        # "ruby_binary" triggers Ruby binary-specific build logic:
        #   - bundle install for dependencies
        #   - Tests via minitest/rspec if present
        #   - Entry point validation
        "rule": "ruby_binary",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "entry_point": entry_point,
        "commands": [
            {"type": "cmd", "program": "bundle", "args": ["install", "--quiet"]},
            {"type": "cmd", "program": "bundle", "args": ["exec", "rake", "test"]},
        ],
    }
