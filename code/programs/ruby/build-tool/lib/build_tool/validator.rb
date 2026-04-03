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
      "perl"
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

    def languages_needing_ci_toolchains(packages)
      packages
        .map(&:language)
        .select { |lang| CI_MANAGED_TOOLCHAIN_LANGUAGES.include?(lang) }
        .uniq
        .sort
    end
  end
end
