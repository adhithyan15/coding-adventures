# frozen_string_literal: true

require "pathname"
require "set"

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
                 "steps.toolchains for: #{missing_output_binding.join(', ')}"
      end
      unless missing_main_force.empty?
        parts << "forced main full-build path does not explicitly enable toolchains for: " \
                 "#{missing_main_force.join(', ')}"
      end

      "#{ci_path.to_s.tr('\\', '/')}: #{parts.join('; ')}"
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

    def validate_lua_isolated_build_files(packages)
      packages.filter_map do |pkg|
        next unless pkg.language == "lua"

        self_rock = "coding-adventures-#{pkg.path.basename.to_s.tr('_', '-')}"
        build_lines = {}
        lua_build_files(pkg.path).flat_map do |build_path|
          lines = read_build_lines(build_path)
          build_lines[build_path.basename.to_s] = lines
          next [] if lines.empty?

          errors = []

          foreign_remove = first_foreign_lua_remove(lines, self_rock)
          unless foreign_remove.nil?
            errors << "#{build_path.to_s.tr('\\', '/')}: Lua BUILD removes unrelated rock " \
                      "#{foreign_remove}; isolated package builds should only remove the " \
                      "package they are rebuilding"
          end

          state_machine_index = first_line_containing(lines, "../state_machine", "..\\state_machine")
          directed_graph_index = first_line_containing(lines, "../directed_graph", "..\\directed_graph")
          if !state_machine_index.nil? && !directed_graph_index.nil? &&
             state_machine_index < directed_graph_index
            errors << "#{build_path.to_s.tr('\\', '/')}: Lua BUILD installs state_machine " \
                      "before directed_graph; isolated LuaRocks builds require directed_graph first"
          end

          if (guarded_local_lua_install?(lines) ||
              (build_path.basename.to_s == "BUILD_windows" && local_lua_sibling_install?(lines))) &&
             !self_install_disables_deps?(lines, self_rock)
            errors << "#{build_path.to_s.tr('\\', '/')}: Lua BUILD bootstraps sibling rocks " \
                      "but the final self-install does not pass --deps-mode=none or --no-manifest"
          end

          errors
        end.then do |errors|
          missing_windows_deps = missing_lua_sibling_installs(
            build_lines.fetch("BUILD", []),
            build_lines.fetch("BUILD_windows", [])
          )
          unless missing_windows_deps.empty?
            errors << "#{(pkg.path / 'BUILD_windows').to_s.tr('\\', '/')}: Lua BUILD_windows is " \
                      "missing sibling installs present in BUILD: #{missing_windows_deps.join(', ')}"
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

          "#{build_path.to_s.tr('\\', '/')}: Perl BUILD bootstraps Test2::V0 without --notest; " \
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
