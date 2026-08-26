# frozen_string_literal: true

require "pathname"
require "json"
require_relative "tracked_artifact_unicode17"

module BuildTool
  module Validator
    module_function

    CI_MANAGED_TOOLCHAIN_LANGUAGES = Set[
      "python",
      "ruby",
      "typescript",
      "rust",
      "elixir",
      "lua",
      "perl",
      "java",
      "kotlin",
      "haskell"
    ].freeze
    TRACKED_ARTIFACT_COMPONENT_IDENTITY = "node_modules"
    TRACKED_ARTIFACT_REDACTED_PATH = "repository"
    TRACKED_ARTIFACT_UNICODE_VERSION = TrackedArtifactUnicode17::UNICODE_VERSION
    ORPHAN_SCAN_ROOT = "code"
    ORPHAN_LEDGER_PATH = "code/BUILD-EXEMPTIONS"
    ORPHAN_BUILD_NAMES = %w[
      BUILD
      BUILD_windows
      BUILD_mac
      BUILD_linux
      BUILD_mac_and_linux
    ].freeze
    ORPHAN_SKIP_COMPONENTS = Set[
      ".git",
      "target",
      "node_modules",
      "vendor",
      ".venv",
      "_build",
      "deps",
      ".build",
      "dist-newstyle",
      ".cargo"
    ].freeze
    PYTHON_BLANK_CODEPOINTS = Set[
      *(0x0009..0x000D),
      *(0x001C..0x0020),
      0x0085,
      0x00A0,
      0x1680,
      *(0x2000..0x200A),
      0x2028,
      0x2029,
      0x202F,
      0x205F,
      0x3000
    ].freeze
    WINDOWS_RESERVED_BASENAMES = Set[
      "CON",
      "PRN",
      "AUX",
      "NUL",
      "CONIN$",
      "CONOUT$",
      "CLOCK$",
      *(1..9).map { |index| "COM#{index}" },
      *(1..9).map { |index| "LPT#{index}" },
      *%w[¹ ² ³].map { |index| "COM#{index}" },
      *%w[¹ ² ³].map { |index| "LPT#{index}" }
    ].freeze

    # Validate caller-supplied records rather than discovering paths here.
    #
    # This boundary is intentionally pure: the native adapter that snapshots a
    # Git index can be reviewed separately, while this policy code never opens a
    # file, follows a link, launches a process, or consults host path semantics.
    def validate_tracked_artifact_snapshot(
      entries,
      unicode_version: TRACKED_ARTIFACT_UNICODE_VERSION
    )
      unless unicode_version == TRACKED_ARTIFACT_UNICODE_VERSION
        raise ArgumentError,
          "tracked artifact Unicode version must be #{TRACKED_ARTIFACT_UNICODE_VERSION}"
      end

      diagnostics = entries.filter_map do |entry|
        normalized_path, problem = normalize_tracked_artifact_path(entry.fetch("path"))
        details = {
          "ordinal" => entry.fetch("ordinal"),
          "entry_kind" => entry.fetch("entry_kind")
        }
        unless problem.nil?
          details["problem"] = problem
          next {
            "code" => "TRACKED_ARTIFACT_PATH_INVALID",
            "severity" => "error",
            "path" => TRACKED_ARTIFACT_REDACTED_PATH,
            "details" => details
          }
        end

        forbidden = normalized_path.split("/").any? do |component|
          TrackedArtifactUnicode17.nfkc_casefold(component) ==
            TRACKED_ARTIFACT_COMPONENT_IDENTITY
        end
        next unless forbidden

        {
          "code" => "TRACKED_ARTIFACT_FORBIDDEN",
          "severity" => "error",
          "path" => normalized_path,
          "details" => details
        }
      end

      diagnostics.sort_by do |diagnostic|
        [
          diagnostic.fetch("code").codepoints,
          diagnostic.fetch("path").codepoints,
          canonical_details(diagnostic.fetch("details"))
        ]
      end
    end

    # Validate a closed Cargo/BUILD/ledger snapshot without touching the host.
    #
    # Discovery belongs to the native Go front door. This adapter deliberately
    # accepts only inert Hash and Array values so every language can implement
    # and test identical policy without gaining filesystem, Git, process,
    # environment, or network authority.
    def validate_orphan_crate_snapshot(snapshot)
      manifests = snapshot.fetch("manifests").reject do |manifest|
        orphan_artifact_path?(manifest.fetch("path"))
      end
      directories = snapshot.fetch("directories").to_set
      manifest_by_path = manifests.to_h { |manifest| [manifest.fetch("path"), manifest] }
      coverage = manifests.to_h do |manifest|
        path = manifest.fetch("path")
        [path, covering_orphan_build(snapshot.fetch("build_files"), path, "runnable")]
      end
      empty_builds = manifests.to_h do |manifest|
        path = manifest.fetch("path")
        [path, covering_orphan_build(snapshot.fetch("build_files"), path, "empty")]
      end

      diagnostics = []
      seen_exemption_paths = Set.new
      valid_exemptions = []

      # Reserve portable identities before field-policy precedence. An invalid
      # first spelling must not let a later full-fold alias escape duplicate
      # detection.
      snapshot.fetch("exemptions").each do |exemption|
        path = exemption.fetch("path")
        identity = nil
        path_problem = if portable_orphan_path?(path)
          identity = orphan_path_identity(path)
          if !under_orphan_scan_root?(path)
            "PATH_OUTSIDE_SCAN"
          elsif orphan_artifact_path?(path)
            "PATH_ARTIFACT"
          end
        else
          "PATH_UNSAFE"
        end

        duplicate = !identity.nil? && seen_exemption_paths.include?(identity)
        seen_exemption_paths.add(identity) unless identity.nil? || duplicate

        problem = if !%w[EXCLUDED PENDING].include?(exemption.fetch("kind"))
          "UNKNOWN_KIND"
        elsif python_blank?(exemption.fetch("reason"))
          "REASON_MISSING"
        elsif duplicate
          "DUPLICATE_PATH"
        else
          path_problem
        end

        unless problem.nil?
          diagnostics << {
            "code" => "ORPHAN_EXEMPTION_INVALID",
            "severity" => "error",
            "path" => ORPHAN_LEDGER_PATH,
            "details" => {"line" => exemption.fetch("line"), "problem" => problem}
          }
          next
        end
        valid_exemptions << exemption
      end

      active_exemptions = {}
      pending_exemption_count = 0
      valid_exemptions.each do |exemption|
        path = exemption.fetch("path")
        stale_problem = if !directories.include?(path)
          "MISSING_DIRECTORY"
        elsif !manifest_by_path.key?(path)
          "NO_MANIFEST"
        elsif !coverage.fetch(path).nil?
          "COVERED"
        end

        unless stale_problem.nil?
          diagnostics << {
            "code" => "ORPHAN_EXEMPTION_STALE",
            "severity" => "error",
            "path" => ORPHAN_LEDGER_PATH,
            "details" => {
              "entry_path" => path,
              "kind" => exemption.fetch("kind"),
              "line" => exemption.fetch("line"),
              "problem" => stale_problem
            }
          }
          next
        end

        active_exemptions[path] = exemption
        pending_exemption_count += 1 if exemption.fetch("kind") == "PENDING"
      end

      manifests.each do |manifest|
        path = manifest.fetch("path")
        next unless coverage.fetch(path).nil? && !active_exemptions.key?(path)

        diagnostic = if empty_builds.fetch(path).nil?
          {
            "code" => "ORPHAN_CRATE_UNLISTED",
            "severity" => "error",
            "path" => path,
            "details" => {"manifest_kind" => manifest.fetch("kind")}
          }
        else
          {
            "code" => "ORPHAN_CRATE_EMPTY_BUILD",
            "severity" => "error",
            "path" => path,
            "details" => {
              "build_path" => empty_builds.fetch(path).fetch("path"),
              "manifest_kind" => manifest.fetch("kind")
            }
          }
        end
        diagnostics << diagnostic
      end

      diagnostics.sort_by! do |diagnostic|
        [
          diagnostic.fetch("code").codepoints,
          diagnostic.fetch("path").codepoints,
          [],
          canonical_details(diagnostic.fetch("details"))
        ]
      end
      {
        "valid" => diagnostics.empty?,
        "diagnostic_codes" => diagnostics.map { |diagnostic| diagnostic.fetch("code") }.uniq.sort,
        "pending_exemption_count" => pending_exemption_count,
        "diagnostics" => diagnostics
      }
    end

    def validate_ci_full_build_toolchains(root, packages)
      ci_path = Pathname(root) / ".github" / "workflows" / "ci.yml"
      return nil unless ci_path.exist?

      workflow = ci_path.read
      return nil unless workflow.include?("Full build on main merge")

      compact_workflow = workflow.gsub(/\s+/, "")
      missing_output_binding = []
      missing_main_force = []

      languages_needing_ci_toolchains(packages).each do |lang|
        output_binding = "needs_#{lang}:${{steps.toolchains.outputs.needs_#{lang}}}"
        missing_output_binding << lang unless compact_workflow.include?(output_binding)
        missing_main_force << lang unless compact_workflow.include?("needs_#{lang}=true")
      end

      return nil if missing_output_binding.empty? && missing_main_force.empty?

      parts = []
      unless missing_output_binding.empty?
        parts << "detect outputs for forced main full builds are not normalized through " \
                 "steps.toolchains for: #{missing_output_binding.join(", ")}"
      end
      unless missing_main_force.empty?
        parts << "forced main full-build path does not explicitly enable toolchains for: " \
                 "#{missing_main_force.join(", ")}"
      end

      "#{ci_path.to_s.tr("\\", "/")}: #{parts.join("; ")}"
    end

    def validate_build_contracts(root, packages)
      errors = []

      ci_error = validate_ci_full_build_toolchains(root, packages)
      errors << ci_error unless ci_error.nil?
      errors.concat(validate_lua_isolated_build_files(packages))
      errors.concat(validate_perl_build_files(packages))

      return nil if errors.empty?

      errors.join("\n  - ")
    end

    def languages_needing_ci_toolchains(packages)
      packages
        .map(&:language)
        .select { |lang| CI_MANAGED_TOOLCHAIN_LANGUAGES.include?(lang) }
        .uniq
        .sort
    end

    # Replace separators lexically. Pathname would erase empty and dot segments
    # before policy can reject them, and would inherit the current host's rules.
    def normalize_tracked_artifact_path(path)
      normalized = path.tr("\\", "/")
      return [nil, "EMPTY"] if normalized.empty?
      return [nil, "TOO_LONG"] if normalized.length > 512
      return [nil, "NON_NFC"] unless TrackedArtifactUnicode17.nfc(normalized) == normalized
      return [nil, "ABSOLUTE"] if normalized.start_with?("/")
      return [nil, "DRIVE_QUALIFIED"] if normalized.match?(/\A[A-Za-z]:/)

      segments = normalized.split("/", -1)
      return [nil, "EMPTY_SEGMENT"] if segments.any?(&:empty?)
      if normalized.each_char.any? { |character| character.ord < 32 || '<>:"|?*'.include?(character) }
        return [nil, "UNSAFE_CHARACTER"]
      end

      segments.each do |segment|
        return [nil, "DOT_SEGMENT"] if [".", ".."].include?(segment)
        return [nil, "TRAILING_DOT_OR_SPACE"] if segment.end_with?(" ", ".")

        basename = TrackedArtifactUnicode17.full_uppercase(segment.split(".", 2).first)
        return [nil, "RESERVED_BASENAME"] if WINDOWS_RESERVED_BASENAMES.include?(basename)
      end
      [normalized, nil]
    end

    def canonical_details(details)
      JSON.generate(details.keys.sort.to_h { |key| [key, details.fetch(key)] }, ascii_only: true)
    end

    def covering_orphan_build(build_files, manifest_path, state)
      build_name_rank = ORPHAN_BUILD_NAMES.each_with_index.to_h
      candidates = build_files.select do |build_file|
        next false unless build_file.fetch("state") == state

        parent, separator, name = build_file.fetch("path").rpartition("/")
        next false if separator.empty? || !under_orphan_scan_root?(parent)
        next false unless manifest_path == parent || manifest_path.start_with?("#{parent}/")

        build_name_rank.key?(name)
      end
      candidates.min_by do |build_file|
        parent, = build_file.fetch("path").rpartition("/")
        name = build_file.fetch("path").rpartition("/").last
        [-parent.split("/").length, build_name_rank.fetch(name), build_file.fetch("path").codepoints]
      end
    end

    def portable_orphan_path?(path)
      return false unless path.is_a?(String) && path.valid_encoding?
      return false if path.empty? || path.codepoints.length > 512
      return false unless TrackedArtifactUnicode17.nfc(path) == path
      return false if path.start_with?("/") || path.include?("\\") || path.include?("//")
      return false if path.match?(/\A[A-Za-z]:/)
      return false if path.each_codepoint.any? { |scalar| scalar < 32 }
      return false if path.each_char.any? { |character| '<>:"|?*'.include?(character) }

      path.split("/", -1).all? do |component|
        next false if component.empty? || %w[. ..].include?(component)
        next false if component.end_with?(" ", ".")

        basename = TrackedArtifactUnicode17.full_uppercase(component.split(".", 2).first)
        !WINDOWS_RESERVED_BASENAMES.include?(basename)
      end
    rescue ArgumentError, EncodingError
      false
    end

    def orphan_path_identity(path)
      TrackedArtifactUnicode17.casefold(TrackedArtifactUnicode17.nfc(path))
    end

    def under_orphan_scan_root?(path)
      path == ORPHAN_SCAN_ROOT || path.start_with?("#{ORPHAN_SCAN_ROOT}/")
    end

    def orphan_artifact_path?(path)
      path.split("/", -1).any? { |component| ORPHAN_SKIP_COMPONENTS.include?(component) }
    end

    def python_blank?(value)
      value.is_a?(String) && value.valid_encoding? &&
        value.each_codepoint.all? { |scalar| PYTHON_BLANK_CODEPOINTS.include?(scalar) }
    end

    def validate_lua_isolated_build_files(packages)
      packages.filter_map do |pkg|
        next unless pkg.language == "lua"

        self_rock = "coding-adventures-#{pkg.path.basename.to_s.tr("_", "-")}"
        build_lines = {}
        lua_build_files(pkg.path).flat_map do |build_path|
          lines = read_build_lines(build_path)
          build_lines[build_path.basename.to_s] = lines
          next [] if lines.empty?

          errors = []

          foreign_remove = first_foreign_lua_remove(lines, self_rock)
          unless foreign_remove.nil?
            errors << "#{build_path.to_s.tr("\\", "/")}: Lua BUILD removes unrelated rock " \
                      "#{foreign_remove}; isolated package builds should only remove the " \
                      "package they are rebuilding"
          end

          state_machine_index = first_line_containing(lines, "../state_machine", "..\\state_machine")
          directed_graph_index = first_line_containing(lines, "../directed_graph", "..\\directed_graph")
          if !state_machine_index.nil? && !directed_graph_index.nil? &&
              state_machine_index < directed_graph_index
            errors << "#{build_path.to_s.tr("\\", "/")}: Lua BUILD installs state_machine " \
                      "before directed_graph; isolated LuaRocks builds require directed_graph first"
          end

          if (guarded_local_lua_install?(lines) ||
              (build_path.basename.to_s == "BUILD_windows" && local_lua_sibling_install?(lines))) &&
              !self_install_disables_deps?(lines, self_rock)
            errors << "#{build_path.to_s.tr("\\", "/")}: Lua BUILD bootstraps sibling rocks " \
                      "but the final self-install does not pass --deps-mode=none or --no-manifest"
          end

          errors
        end.then do |errors|
          missing_windows_deps = missing_lua_sibling_installs(
            build_lines.fetch("BUILD", []),
            build_lines.fetch("BUILD_windows", [])
          )
          unless missing_windows_deps.empty?
            errors << "#{(pkg.path / "BUILD_windows").to_s.tr("\\", "/")}: Lua BUILD_windows is " \
                      "missing sibling installs present in BUILD: #{missing_windows_deps.join(", ")}"
          end
          errors
        end
      end.flatten
    end

    def validate_perl_build_files(packages)
      packages.filter_map do |pkg|
        next unless pkg.language == "perl"

        lua_build_files(pkg.path).filter_map do |build_path|
          lines = read_build_lines(build_path)
          next unless lines.any? do |line|
            line.include?("cpanm") &&
              line.include?("Test2::V0") &&
              !line.include?("--notest")
          end

          "#{build_path.to_s.tr("\\", "/")}: Perl BUILD bootstraps Test2::V0 without --notest; " \
            "isolated Windows installs can fail while installing the test framework itself"
        end
      end.flatten
    end

    def lua_build_files(pkg_path)
      Dir.children(pkg_path)
        .select { |entry| entry.start_with?("BUILD") }
        .sort
        .map { |entry| Pathname(pkg_path) / entry }
    rescue SystemCallError
      []
    end

    def read_build_lines(build_path)
      return [] unless build_path.exist?

      build_path.read
        .lines
        .map(&:strip)
        .reject { |line| line.empty? || line.start_with?("#") }
    end

    def first_foreign_lua_remove(lines, self_rock)
      lines.each do |line|
        match = line.match(/\bluarocks remove --force ([^ \t]+)/)
        next if match.nil? || match[1] == self_rock

        return match[1]
      end
      nil
    end

    def first_line_containing(lines, *needles)
      lines.each_with_index do |line, index|
        return index if needles.any? { |needle| line.include?(needle) }
      end
      nil
    end

    def guarded_local_lua_install?(lines)
      lines.any? do |line|
        line.include?("luarocks show ") && (line.include?("../") || line.include?("..\\"))
      end
    end

    def local_lua_sibling_install?(lines)
      !lua_sibling_install_dirs(lines).empty?
    end

    def self_install_disables_deps?(lines, self_rock)
      lines.any? do |line|
        line.include?("luarocks make") &&
          line.include?(self_rock) &&
          (line.include?("--deps-mode=none") ||
            line.include?("--deps-mode none") ||
            line.include?("--no-manifest"))
      end
    end

    def missing_lua_sibling_installs(unix_lines, windows_lines)
      windows_deps = lua_sibling_install_dirs(windows_lines).to_set
      lua_sibling_install_dirs(unix_lines).reject { |dep| windows_deps.include?(dep) }
    end

    def lua_sibling_install_dirs(lines)
      lines.filter_map do |line|
        next unless line.include?("luarocks make")

        match = line.match(/\bcd\s+([.][.][\\\/][^ \t\r\n&()]+)/)
        match && match[1].tr("\\", "/")
      end.uniq.sort
    end
  end
end
