# frozen_string_literal: true

require_relative "test_helper"

# Package metadata is executable Ruby too. Pin its actual payload rather than
# relying on a README claim that a source checkout can be published.
class MatrixPackagingTest < Minitest::Test
  def test_repository_pending_exemption_is_retired
    # A BUILD front and a PENDING exemption are contradictory. The production
    # planner rejects that stale exemption before any package tests can run.
    exemptions = File.expand_path("../../../../BUILD-EXEMPTIONS", __dir__)
    entries = File.readlines(exemptions).reject { |line| line.strip.start_with?("#") }
    refute entries.any? { |line| line.split[1] == "code/packages/ruby/matrix" }
  end

  def setup
    @root = File.expand_path("..", __dir__)
    @spec = Gem::Specification.load(File.join(@root, "coding_adventures_matrix.gemspec"))
  end

  def test_gem_identity_and_exact_runtime_payload
    assert_equal "coding_adventures_matrix", @spec.name
    assert_equal "0.2.0", @spec.version.to_s
    assert_equal "MIT", @spec.license
    assert_equal ["lib"], @spec.require_paths
    assert_empty @spec.runtime_dependencies
    assert_empty @spec.extensions
    assert_equal %w[lib/matrix_ml.rb README.md CHANGELOG.md LICENSE required_capabilities.json].sort, @spec.files.sort
    @spec.files.each { |path| assert File.file?(File.join(@root, path)), path }
  end

  def test_platform_fronts_keep_lint_coverage_and_installed_smoke
    common = ["bundle install", "bundle exec rake"]
    unix = File.readlines(File.join(@root, "BUILD"), chomp: true)
    windows = File.readlines(File.join(@root, "BUILD_windows"), chomp: true)
    assert_equal [common[0], "bundle exec standardrb", common[1]], unix
    assert_equal [common[0], "bundle exec standardrb --except Layout/EndOfLine", common[1]], windows
    rake = File.read(File.join(@root, "Rakefile"))
    assert_includes rake, "scripts/gem_smoke.rb"
    assert_includes rake, "task default: [:test, :package_smoke]"
    helper = File.read(File.join(@root, "test/test_helper.rb"))
    assert_includes helper, "minimum_coverage 95"
    assert_includes helper, 'track_files "lib/**/*.rb"'
  end
end
