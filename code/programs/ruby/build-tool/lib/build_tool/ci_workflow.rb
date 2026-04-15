# frozen_string_literal: true

require "open3"
require "set"

module BuildTool
  module CIWorkflow
    module_function

    CI_WORKFLOW_PATH = ".github/workflows/ci.yml"
    ALL_TOOLCHAINS = %w[
      python ruby go typescript rust elixir lua perl swift java kotlin haskell dotnet
    ].freeze

    Change = Data.define(:toolchains, :requires_full_rebuild)

    TOOLCHAIN_MARKERS = {
      "python" => [
        "needs_python", "setup-python", "python-version", "setup-uv",
        "python --version", "uv --version", "pytest",
        "set up python", "install uv"
      ],
      "ruby" => [
        "needs_ruby", "setup-ruby", "ruby-version", "bundler",
        "gem install bundler", "ruby --version", "bundle --version",
        "set up ruby", "install bundler"
      ],
      "go" => [
        "needs_go", "setup-go", "go-version", "go version", "set up go"
      ],
      "typescript" => [
        "needs_typescript", "setup-node", "node-version", "npm install -g jest",
        "node --version", "npm --version", "set up node"
      ],
      "rust" => [
        "needs_rust", "rust-toolchain", "cargo", "rustc", "tarpaulin",
        "wasm32-unknown-unknown", "set up rust", "install cargo-tarpaulin"
      ],
      "elixir" => [
        "needs_elixir", "setup-beam", "elixir-version", "otp-version",
        "elixir --version", "mix --version", "set up elixir"
      ],
      "lua" => [
        "needs_lua", "gh-actions-lua", "gh-actions-luarocks", "luarocks",
        "lua -v", "msvc", "set up lua", "set up luarocks"
      ],
      "perl" => [
        "needs_perl", "cpanm", "perl --version", "install cpanm"
      ],
      "haskell" => [
        "needs_haskell", "haskell-actions/setup", "ghc-version", "cabal-version",
        "ghc --version", "cabal --version", "set up haskell"
      ],
      "java" => [
        "needs_java", "setup-java", "java-version", "java --version",
        "temurin", "set up jdk", "set up gradle", "setup-gradle",
        "disable long-lived gradle services",
        "gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch"
      ],
      "kotlin" => [
        "needs_kotlin", "setup-java", "java-version",
        "temurin", "set up jdk", "set up gradle", "setup-gradle",
        "disable long-lived gradle services",
        "gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch"
      ],
      "dotnet" => [
        "needs_dotnet", "setup-dotnet", "dotnet-version", "dotnet --version",
        "set up .net"
      ]
    }.freeze

    UNSAFE_MARKERS = %w[
      ./build-tool build-tool.exe -detect-languages -emit-plan -force -plan-file
      -validate-build-files actions/checkout build-plan cancel-in-progress:
      concurrency: diff-base download-artifact event_name fetch-depth
      git_ref is_main matrix: permissions: pr_base_ref pull_request:
      push: runs-on: strategy: upload-artifact
    ].freeze + ["git fetch origin main"]

    def analyze_changes(root, diff_base)
      analyze_patch(file_diff(root, diff_base, CI_WORKFLOW_PATH))
    end

    def analyze_patch(patch)
      toolchains = Set.new
      hunk = []

      flush = lambda do
        hunk_toolchains, unsafe = classify_hunk(hunk)
        hunk = []
        return Change.new(Set.new.freeze, true) if unsafe

        toolchains.merge(hunk_toolchains)
        nil
      end

      patch.each_line do |line|
        if line.start_with?("@@")
          result = flush.call
          return result if result
          next
        end
        next if line.start_with?("diff --git ", "index ", "--- ", "+++ ")

        hunk << line.chomp
      end

      result = flush.call
      return result if result

      Change.new(toolchains.freeze, false)
    end

    def sorted_toolchains(toolchains)
      toolchains.to_a.sort
    end

    def toolchain_for_package_language(language)
      case language
      when "wasm" then "rust"
      when "csharp", "fsharp", "dotnet" then "dotnet"
      else language
      end
    end

    def compute_languages_needed(packages, affected_set, force, ci_toolchains = Set.new)
      needed = ALL_TOOLCHAINS.each_with_object({}) { |lang, acc| acc[lang] = false }
      needed["go"] = true

      if force || affected_set.nil?
        ALL_TOOLCHAINS.each { |lang| needed[lang] = true }
        return needed
      end

      packages.each do |pkg|
        needed[toolchain_for_package_language(pkg.language)] = true if affected_set.key?(pkg.name)
      end

      ci_toolchains.each { |toolchain| needed[toolchain] = true }

      needed
    end

    def classify_hunk(lines)
      hunk_toolchains = Set.new
      changed_toolchains = Set.new
      changed_lines = []

      lines.each do |line|
        next if line.empty? || !diff_line?(line)

        content = line[1..]&.strip.to_s
        hunk_toolchains.merge(detect_toolchains(content))

        next unless changed_line?(line)
        next if content.empty? || content.start_with?("#")

        changed_lines << content
        changed_toolchains.merge(detect_toolchains(content))
      end

      return [Set.new, false] if changed_lines.empty?

      resolved_toolchains = changed_toolchains
      if resolved_toolchains.empty?
        return [Set.new, true] unless hunk_toolchains.size == 1

        resolved_toolchains = hunk_toolchains
      end

      changed_lines.each do |content|
        return [Set.new, true] if touches_shared_ci_behavior?(content)
        next unless detect_toolchains(content).empty?
        next if toolchain_scoped_structural_line?(content)

        return [Set.new, true]
      end

      [resolved_toolchains, false]
    end

    def detect_toolchains(content)
      normalized = content.downcase
      TOOLCHAIN_MARKERS.each_with_object(Set.new) do |(toolchain, markers), found|
        found << toolchain if markers.any? { |marker| normalized.include?(marker) }
      end
    end

    def touches_shared_ci_behavior?(content)
      normalized = content.downcase
      UNSAFE_MARKERS.any? { |marker| normalized.include?(marker) }
    end

    def toolchain_scoped_structural_line?(content)
      content.start_with?(
        "if:", "run:", "shell:", "with:", "env:", "{", "}", "else", "fi", "then",
        "printf ", "echo ", "curl ", "powershell ", "call ", "cd "
      )
    end

    def diff_line?(line)
      line.start_with?(" ") || changed_line?(line)
    end

    def changed_line?(line)
      line.start_with?("+", "-")
    end

    def file_diff(root, diff_base, relative_path)
      [
        ["git", "diff", "--unified=0", "#{diff_base}...HEAD", "--", relative_path],
        ["git", "diff", "--unified=0", diff_base, "HEAD", "--", relative_path]
      ].each do |cmd|
        stdout, status = Open3.capture2(*cmd, chdir: root.to_s)
        return stdout if status.success?
      end
      ""
    end
  end
end
