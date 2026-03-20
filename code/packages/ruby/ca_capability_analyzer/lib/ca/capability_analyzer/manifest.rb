# frozen_string_literal: true

require "json"

# ============================================================================
# Manifest — Loading and Comparing Declared vs Detected Capabilities
# ============================================================================
#
# This module loads a package's `required_capabilities.json` manifest and
# compares it against the capabilities detected by the analyzer.
#
# ## The Core Question
#
# The comparison answers: "Does this package use only the capabilities
# it declared?" This is the heart of the capability security system.
#
# ## Comparison Logic
#
# The comparison is **asymmetric** — different results have different
# severity levels:
#
# - **Undeclared capabilities** (detected but not in manifest) are ERRORS.
#   The code uses something it didn't declare. This is a security
#   violation — the package is accessing resources it didn't ask for.
#
# - **Unused declarations** (in manifest but not detected) are WARNINGS.
#   The manifest declares a capability the code doesn't use. This isn't
#   a security issue — it's just a stale declaration that should be
#   cleaned up.
#
# ## Default Deny
#
# If no `required_capabilities.json` exists, the package is treated as
# having zero declared capabilities. Any detected capability is an error.
# This is the "no manifest = block everything" principle.
#
# ## Target Matching
#
# When comparing detected targets against declared targets, we use
# glob-style matching (via File.fnmatch):
#
#   - `*` matches anything
#   - `*.yml` matches `config.yml`
#   - Exact strings match exactly
#
# This mirrors OpenBSD's `unveil()` path matching.
# ============================================================================

module CA
  module CapabilityAnalyzer
    # A parsed required_capabilities.json manifest.
    #
    # This is the "contract" a package signs: "I need these capabilities
    # and no others." The CI gate enforces this contract.
    Manifest = Struct.new(
      :package,                      # Qualified package name
      :capabilities,                 # Array of declared capability hashes
      :justification,                # Human-readable justification
      :banned_construct_exceptions,  # Array of exempted banned constructs
      :path,                         # Path to the manifest file (if loaded)
      keyword_init: true
    ) do
      def empty?
        capabilities.nil? || capabilities.empty?
      end
    end

    # Result of comparing detected capabilities against a manifest.
    #
    # This is the "verdict" — did the package pass or fail the
    # capability check?
    ComparisonResult = Struct.new(
      :passed,   # Boolean — true if all detected capabilities are declared
      :errors,   # Array of undeclared DetectedCapability (violations)
      :warnings, # Array of unused declaration hashes (stale entries)
      :matched,  # Array of matched DetectedCapability
      keyword_init: true
    ) do
      def summary
        lines = []

        if passed
          lines << "PASS -- all detected capabilities are declared."
        else
          lines << "FAIL -- #{errors.length} undeclared capability(ies) detected."
        end

        if errors && !errors.empty?
          lines << ""
          lines << "Undeclared capabilities (ERRORS):"
          errors.each do |cap|
            lines << "  #{cap.file}:#{cap.line}: #{cap} (#{cap.evidence})"
          end
        end

        if warnings && !warnings.empty?
          lines << ""
          lines << "Unused declarations (WARNINGS):"
          warnings.each do |decl|
            lines << "  #{decl["category"]}:#{decl["action"]}:#{decl["target"]}"
          end
        end

        if matched && !matched.empty?
          lines << ""
          lines << "Matched: #{matched.length} capability(ies)."
        end

        lines.join("\n")
      end
    end

    # Load a manifest from a JSON file.
    #
    # @param path [String] path to required_capabilities.json.
    # @return [Manifest] parsed manifest.
    # @raise [Errno::ENOENT] if the file does not exist.
    # @raise [JSON::ParserError] if the file is not valid JSON.
    def self.load_manifest(path)
      data = JSON.parse(File.read(path))
      Manifest.new(
        package: data["package"],
        capabilities: data.fetch("capabilities", []),
        justification: data.fetch("justification", ""),
        banned_construct_exceptions: data.fetch("banned_construct_exceptions", []),
        path: path
      )
    end

    # Create a default (empty) manifest for a package without one.
    #
    # This represents the "no manifest = default deny" policy. A package
    # without a required_capabilities.json is treated as declaring zero
    # capabilities — any detected capability is a violation.
    #
    # @param package_name [String] the package name.
    # @return [Manifest] empty manifest.
    def self.default_manifest(package_name)
      Manifest.new(
        package: package_name,
        capabilities: [],
        justification: "No manifest file -- default deny (zero capabilities).",
        banned_construct_exceptions: [],
        path: nil
      )
    end

    # ── Target Matching ──────────────────────────────────────────────
    #
    # Check if a detected target matches a declared target pattern.
    # Uses File.fnmatch for glob-style matching:
    #
    #   - "*" matches anything
    #   - "*.yml" matches "config.yml"
    #   - "file.txt" matches "file.txt" exactly
    #
    # Special case: if the detected target is "*" (unknown/any), it
    # matches any declared pattern. This is conservative — we accept
    # it rather than false-positive, since we can't know what the
    # runtime value will be.

    def self.target_matches?(pattern, actual)
      return true if pattern == "*"
      return true if actual == "*"

      File.fnmatch(pattern, actual)
    end

    # Check if a detected capability matches a declared one.
    #
    # A match requires:
    # 1. Same category (fs, net, proc, etc.)
    # 2. Compatible action (exact match, or declared is "*")
    # 3. Compatible target (glob match)
    def self.capability_matches?(declared, detected)
      return false unless declared["category"] == detected.category
      return false unless declared["action"] == "*" || declared["action"] == detected.action

      target_matches?(declared["target"], detected.target)
    end

    # Compare detected capabilities against a manifest.
    #
    # This is the core comparison logic used by the CI gate. It
    # determines whether a package's source code uses only the
    # capabilities it declared.
    #
    # @param detected [Array<DetectedCapability>] capabilities found.
    # @param manifest [Manifest] the package's declarations.
    # @return [ComparisonResult] pass/fail with details.
    def self.compare_capabilities(detected, manifest)
      errors = []
      matched = []

      # For each detected capability, check if any declaration covers it.
      detected.each do |cap|
        found_match = manifest.capabilities.any? do |decl|
          capability_matches?(decl, cap)
        end

        if found_match
          matched << cap
        else
          errors << cap
        end
      end

      # Find unused declarations (warnings).
      # A declaration is "used" if at least one detected capability matches it.
      used_indices = Set.new
      detected.each do |cap|
        manifest.capabilities.each_with_index do |decl, i|
          if capability_matches?(decl, cap)
            used_indices << i
            break
          end
        end
      end

      warnings = manifest.capabilities.each_with_index
        .reject { |_decl, i| used_indices.include?(i) }
        .map { |decl, _i| decl }

      ComparisonResult.new(
        passed: errors.empty?,
        errors: errors,
        warnings: warnings,
        matched: matched
      )
    end
  end
end
