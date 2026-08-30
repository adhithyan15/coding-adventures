# frozen_string_literal: true

# toolchain_detection.rb -- Pure extra-CI toolchain snapshot evaluation
# ======================================================================
#
# A BUILD file can ask CI for an extra compiler with an inert comment such as:
#
#     # needs-toolchain: java
#
# The important word is *inert*. This module receives an already bounded
# snapshot and returns a decision. It never opens a BUILD file, probes PATH,
# reads the environment, invokes Git, or starts a process. Keeping that host
# authority outside this boundary makes the same fixture meaningful in every
# build-tool implementation.

module BuildTool
  module ToolchainDetection
    # A sorted registry makes both JSON output and human review deterministic.
    # Freeze the strings as well as the array: freezing only the container would
    # still let one caller mutate a shared key for every later evaluation.
    CANONICAL_TOOLCHAINS = %w[
      cpp
      dart
      dotnet
      elixir
      go
      haskell
      java
      kotlin
      lua
      ocaml
      perl
      python
      ruby
      rust
      swift
      typescript
    ].map!(&:freeze).freeze

    CANONICAL_TOOLCHAIN_SET = CANONICAL_TOOLCHAINS.to_h { |name| [name, true] }.freeze

    # Source buckets sometimes name a language more narrowly than its compiler
    # family. These aliases are decisions over caller data, not host probes.
    LANGUAGE_ALIASES = {
      "c" => "cpp",
      "cpp" => "cpp",
      "csharp" => "dotnet",
      "fsharp" => "dotnet",
      "dotnet" => "dotnet",
      "wasm" => "rust"
    }.transform_keys!(&:freeze).transform_values!(&:freeze).freeze

    FRONT_PRECEDENCE = {
      "windows" => %w[BUILD_windows BUILD],
      "darwin" => %w[BUILD_mac BUILD_mac_and_linux BUILD],
      "linux" => %w[BUILD_linux BUILD_mac_and_linux BUILD]
    }.transform_values { |fronts| fronts.map!(&:freeze).freeze }.freeze

    PER_FILE_BYTE_LIMIT = 65_536
    PER_FILE_LINE_LIMIT = 4_096
    AGGREGATE_BYTE_LIMIT = 1_048_576
    DECLARATION = /\A# needs-toolchain:[ \t]+([a-z]+)[ \t]*\z/

    module_function

    # parse_extra_toolchains -- Read exact declarations from an in-memory string.
    #
    # Ruby's String#strip and String#each_line know about more whitespace and
    # line endings than this protocol permits. Literal LF splitting plus a
    # one-CR terminator rule keeps a lone or doubled CR as ordinary content.
    def parse_extra_toolchains(content)
      utf8 = strict_utf8_copy(content)
      return [] if utf8.bytesize > PER_FILE_BYTE_LIMIT
      return [] if utf8.count("\n") + 1 > PER_FILE_LINE_LIMIT

      lines = utf8.split("\n", -1)
      seen = {}
      declarations = []

      lines.each_with_index do |raw_line, index|
        line = raw_line.dup
        line = line.byteslice(0, line.bytesize - 1) if index < lines.length - 1 && line.end_with?("\r")
        line = trim_ascii_space_and_tab(line)
        match = DECLARATION.match(line)
        next unless match

        name = match[1]
        next unless CANONICAL_TOOLCHAIN_SET.key?(name)
        next if seen.key?(name)

        seen[name] = true
        declarations << name
      end

      declarations
    end

    # evaluate_snapshot -- Evaluate one closed, process-free toolchain snapshot.
    #
    # Resource validation deliberately visits *every* supplied front before
    # selection. An oversized unselected Windows front is still an invalid
    # snapshot on Linux; otherwise adapters could disagree merely because they
    # happened to ignore different hostile inputs.
    def evaluate_snapshot(platform, force_full, packages, scheduled_packages, forced_toolchains)
      platform = platform.to_s
      front_precedence(platform)
      normalized_packages = normalize_and_meter_packages(packages)
      selected_names = if scheduled_packages.nil?
        nil
      else
        scheduled_packages.to_h { |name| [name, true] }
      end

      selected_packages = normalized_packages.select do |package|
        selected_names.nil? || selected_names.key?(package.fetch("name"))
      end

      # Package diagnostics precede forced-toolchain diagnostics. Validate even
      # in force-full mode so a full rebuild cannot hide an unsupported lane.
      language_toolchains = []
      selected_packages.each do |package|
        toolchain = language_toolchain(package.fetch("language"))
        return unsupported(package.fetch("name")) unless toolchain

        language_toolchains << [package, toolchain]
      end

      forced = forced_toolchains.map(&:to_s)
      return unsupported if forced.any? { |name| !CANONICAL_TOOLCHAIN_SET.key?(name) }

      toolchains = fresh_toolchain_map(force_full)
      unless force_full
        language_toolchains.each do |package, language_toolchain|
          toolchains[language_toolchain] = true
          selected_front(package.fetch("build_files"), platform).then do |content|
            parse_extra_toolchains(content).each { |name| toolchains[name] = true }
          end
        end
      end
      forced.each { |name| toolchains[name] = true }

      {
        "outcome" => "ok",
        "toolchains" => toolchains,
        "diagnostics" => []
      }
    end

    # Ruby Strings carry an encoding tag. bytesize alone therefore measures the
    # representation Ruby happens to hold, not the encoded UTF-8 payload shared
    # by the language-neutral contract. Transcode a copy strictly first.
    def strict_utf8_copy(content)
      content.dup.encode(Encoding::UTF_8)
    end
    private_class_method :strict_utf8_copy

    def trim_ascii_space_and_tab(line)
      line.sub(/\A[ \t]+/, "").sub(/[ \t]+\z/, "")
    end
    private_class_method :trim_ascii_space_and_tab

    def normalize_and_meter_packages(packages)
      aggregate_bytes = 0

      packages.map do |package|
        build_files = {}
        package.fetch("build_files").each do |front, raw_content|
          content = strict_utf8_copy(raw_content)
          bytes = content.bytesize
          lines = content.count("\n") + 1
          if bytes > PER_FILE_BYTE_LIMIT || lines > PER_FILE_LINE_LIMIT
            raise ArgumentError, "BUILD front exceeds per-file resource ceiling"
          end

          aggregate_bytes += bytes
          if aggregate_bytes > AGGREGATE_BYTE_LIMIT
            raise ArgumentError, "BUILD snapshot exceeds aggregate resource ceiling"
          end

          build_files[front.to_s] = content
        end

        {
          "name" => package.fetch("name").to_s.dup,
          "language" => package.fetch("language").to_s.dup,
          "build_files" => build_files
        }
      end
    end
    private_class_method :normalize_and_meter_packages

    def language_toolchain(language)
      normalized = LANGUAGE_ALIASES.fetch(language, language)
      CANONICAL_TOOLCHAIN_SET.key?(normalized) ? normalized : nil
    end
    private_class_method :language_toolchain

    def selected_front(build_files, platform)
      front_precedence(platform).each do |front|
        return build_files.fetch(front) if build_files.key?(front)
      end
      ""
    end
    private_class_method :selected_front

    def front_precedence(platform)
      FRONT_PRECEDENCE.fetch(platform) do
        raise ArgumentError, "unsupported target platform: #{platform}"
      end
    end
    private_class_method :front_precedence

    def fresh_toolchain_map(enabled)
      CANONICAL_TOOLCHAINS.to_h { |name| [name, enabled] }
    end
    private_class_method :fresh_toolchain_map

    def unsupported(package_name = nil)
      diagnostic = {
        "code" => "TOOLCHAIN_UNSUPPORTED",
        "severity" => "error"
      }
      diagnostic["package"] = package_name if package_name

      {
        "outcome" => "error",
        "toolchains" => {},
        "diagnostics" => [diagnostic]
      }
    end
    private_class_method :unsupported
  end
end
