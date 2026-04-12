# frozen_string_literal: true

# test_discovery.rb -- Tests for package discovery
# =================================================
#
# These tests verify the recursive BUILD file walking logic, language inference,
# platform-specific BUILD file selection, skip-list filtering, and the overall
# discover_packages pipeline.

require_relative "test_helper"

class TestDiscovery < Minitest::Test
  include TestHelper

  # -- read_lines tests -------------------------------------------------------

  def test_read_lines_returns_non_blank_non_comment_lines
    # read_lines should strip whitespace, skip blank lines, and skip comments.
    dir = create_temp_dir
    file = dir / "test_file"
    write_file(file, "line1\n  line2  \n\n# comment\nline3\n")

    result = BuildTool::Discovery.read_lines(file)
    assert_equal %w[line1 line2 line3], result
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_read_lines_returns_empty_for_missing_file
    result = BuildTool::Discovery.read_lines(Pathname("/nonexistent/file"))
    assert_equal [], result
  end

  def test_read_lines_returns_empty_for_blank_file
    dir = create_temp_dir
    file = dir / "empty"
    write_file(file, "\n\n  \n")

    result = BuildTool::Discovery.read_lines(file)
    assert_equal [], result
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- infer_language tests ----------------------------------------------------

  def test_infer_language_python
    path = Pathname("/repo/code/packages/python/logic-gates")
    assert_equal "python", BuildTool::Discovery.infer_language(path)
  end

  def test_infer_language_ruby
    path = Pathname("/repo/code/packages/ruby/logic_gates")
    assert_equal "ruby", BuildTool::Discovery.infer_language(path)
  end

  def test_infer_language_go
    path = Pathname("/repo/code/programs/go/build-tool")
    assert_equal "go", BuildTool::Discovery.infer_language(path)
  end

  def test_infer_language_rust
    path = Pathname("/repo/code/packages/rust/logic-gates")
    assert_equal "rust", BuildTool::Discovery.infer_language(path)
  end

  def test_infer_language_unknown
    path = Pathname("/repo/code/packages/haskell/something")
    assert_equal "haskell", BuildTool::Discovery.infer_language(path)
  end

  # -- infer_package_name tests ------------------------------------------------

  def test_infer_package_name
    path = Pathname("/repo/code/packages/python/logic-gates")
    assert_equal "python/logic-gates", BuildTool::Discovery.infer_package_name(path, "python")
  end

  # -- get_build_file tests ----------------------------------------------------

  def test_get_build_file_returns_generic_build
    dir = create_temp_dir
    build = dir / "BUILD"
    write_file(build, "echo hello")

    result = BuildTool::Discovery.get_build_file(dir)
    assert_equal build, result
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_get_build_file_returns_nil_when_missing
    dir = create_temp_dir
    result = BuildTool::Discovery.get_build_file(dir)
    assert_nil result
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_get_build_file_prefers_platform_build_on_darwin
    # On macOS (darwin), BUILD_mac should take priority over BUILD.
    skip unless RUBY_PLATFORM.include?("darwin")

    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_mac", "mac-specific")

    result = BuildTool::Discovery.get_build_file(dir)
    assert_equal dir / "BUILD_mac", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  # -- get_build_file_for_platform tests (cross-platform, testable) ----------

  def test_get_build_file_for_platform_mac_preferred
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_mac", "mac")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "darwin")
    assert_equal dir / "BUILD_mac", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_get_build_file_for_platform_linux_preferred
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_linux", "linux")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "linux")
    assert_equal dir / "BUILD_linux", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_get_build_file_for_platform_windows_preferred
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_windows", "windows")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "windows")
    assert_equal dir / "BUILD_windows", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_get_build_file_for_platform_windows_fallback
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "windows")
    assert_equal dir / "BUILD", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_get_build_file_for_platform_windows_not_on_mac
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_windows", "windows")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "darwin")
    assert_equal dir / "BUILD", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_get_build_file_for_platform_mac_and_linux_on_mac
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_mac_and_linux", "unix")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "darwin")
    assert_equal dir / "BUILD_mac_and_linux", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_get_build_file_for_platform_mac_and_linux_on_linux
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_mac_and_linux", "unix")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "linux")
    assert_equal dir / "BUILD_mac_and_linux", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_get_build_file_for_platform_mac_and_linux_not_on_windows
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_mac_and_linux", "unix")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "windows")
    assert_equal dir / "BUILD", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_get_build_file_for_platform_mac_overrides_mac_and_linux
    dir = create_temp_dir
    write_file(dir / "BUILD", "generic")
    write_file(dir / "BUILD_mac", "mac")
    write_file(dir / "BUILD_mac_and_linux", "unix")

    result = BuildTool::Discovery.get_build_file_for_platform(dir, "darwin")
    assert_equal dir / "BUILD_mac", result
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  # -- discover_packages integration tests -------------------------------------

  def test_discover_simple_fixture
    # The simple fixture has one package at pkg-a/ with a BUILD file.
    packages = BuildTool::Discovery.discover_packages(simple_fixture)

    assert_equal 1, packages.size
    pkg = packages.first
    # The fixture lives under .../ruby/build-tool/test/fixtures/simple/pkg-a,
    # so the language inference picks up "ruby" from the path (because the
    # word "ruby" appears in the path components).
    assert_equal "ruby/pkg-a", pkg.name
    assert_equal "ruby", pkg.language
    assert_equal ["echo \"building pkg-a\""], pkg.build_commands
  end

  def test_discover_diamond_fixture
    # The diamond fixture has 4 packages under pkgs/python/.
    packages = BuildTool::Discovery.discover_packages(diamond_fixture)

    assert_equal 4, packages.size
    names = packages.map(&:name).sort
    assert_equal %w[python/pkg-a python/pkg-b python/pkg-c python/pkg-d], names
  end

  def test_discover_packages_returns_sorted
    packages = BuildTool::Discovery.discover_packages(diamond_fixture)
    names = packages.map(&:name)
    assert_equal names.sort, names
  end

  def test_discover_empty_directory
    dir = create_temp_dir
    packages = BuildTool::Discovery.discover_packages(dir)
    assert_equal [], packages
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- recursive discovery tests -----------------------------------------------

  def test_discover_finds_nested_packages_without_dirs_files
    dir = create_temp_dir
    pkg_a = dir / "packages" / "python" / "pkg-a"
    pkg_b = dir / "packages" / "python" / "pkg-b"
    write_file(pkg_a / "BUILD", "echo a")
    write_file(pkg_b / "BUILD", "echo b")

    packages = BuildTool::Discovery.discover_packages(dir)
    assert_equal 2, packages.size
    names = packages.map(&:name)
    assert_includes names, "python/pkg-a"
    assert_includes names, "python/pkg-b"
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_build_stops_recursion
    dir = create_temp_dir
    write_file(dir / "pkg-a" / "BUILD", "echo top")
    write_file(dir / "pkg-a" / "sub" / "BUILD", "echo sub")

    packages = BuildTool::Discovery.discover_packages(dir)
    assert_equal 1, packages.size
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_discover_multi_language
    dir = create_temp_dir
    %w[python ruby go rust].each do |lang|
      write_file(dir / "packages" / lang / "lib" / "BUILD", "echo #{lang}")
    end

    packages = BuildTool::Discovery.discover_packages(dir)
    assert_equal 4, packages.size
    langs = packages.map(&:language).to_set
    assert_equal Set.new(%w[python ruby go rust]), langs
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- skip list tests ---------------------------------------------------------

  def test_discover_skips_git_dir
    dir = create_temp_dir
    write_file(dir / "packages" / "python" / "pkg-a" / "BUILD", "echo a")
    write_file(dir / ".git" / "hooks" / "BUILD", "echo git")

    packages = BuildTool::Discovery.discover_packages(dir)
    assert_equal 1, packages.size
    assert_equal "python/pkg-a", packages.first.name
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_discover_skips_venv_dir
    dir = create_temp_dir
    write_file(dir / "packages" / "python" / "pkg-a" / "BUILD", "echo a")
    write_file(dir / ".venv" / "lib" / "BUILD", "echo venv")

    packages = BuildTool::Discovery.discover_packages(dir)
    assert_equal 1, packages.size
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_discover_skips_node_modules
    dir = create_temp_dir
    write_file(dir / "packages" / "python" / "pkg-a" / "BUILD", "echo a")
    write_file(dir / "node_modules" / "dep" / "BUILD", "echo node")

    packages = BuildTool::Discovery.discover_packages(dir)
    assert_equal 1, packages.size
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_discover_skips_target_dir
    dir = create_temp_dir
    write_file(dir / "packages" / "rust" / "lib" / "BUILD", "echo rs")
    write_file(dir / "target" / "debug" / "BUILD", "echo target")

    packages = BuildTool::Discovery.discover_packages(dir)
    assert_equal 1, packages.size
    assert_equal "rust", packages.first.language
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_package_is_data_define
    # Verify that Package is an immutable Data object.
    pkg = BuildTool::Package.new(
      name: "test/pkg",
      path: Pathname("/tmp"),
      build_commands: ["echo hi"],
      language: "ruby"
    )
    assert_equal "test/pkg", pkg.name
    assert_equal Pathname("/tmp"), pkg.path
    assert_equal ["echo hi"], pkg.build_commands
    assert_equal "ruby", pkg.language
  end
end
