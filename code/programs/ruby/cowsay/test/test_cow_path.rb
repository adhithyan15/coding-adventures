# frozen_string_literal: true

# Tests for cow-file selection -- the path-traversal fix for issue #12169.
#
# Each test builds a throwaway tree like this:
#
#   <tmp>/
#     secret.cow          <- must NEVER be readable via -f
#     cows/
#       default.cow       <- the fallback
#       tux.cow           <- a legitimate cow
#       nested/inner.cow  <- exists, but "nested/inner" is not a bare name
#
# and asserts that every hostile spelling of "secret" draws the default cow.

require "minitest/autorun"
require "fileutils"
require "tmpdir"
require_relative "../cow_path"

class CowPathTest < Minitest::Test
  def setup
    @base = Dir.mktmpdir("cowsay-traversal")
    @cows = File.join(@base, "cows")
    write_cow(File.join(@cows, "default.cow"), "DEFAULT")
    write_cow(File.join(@cows, "tux.cow"), "TUX")
    write_cow(File.join(@cows, "nested", "inner.cow"), "NESTED")
    write_cow(File.join(@base, "secret.cow"), "SECRET")
  end

  def teardown
    FileUtils.remove_entry(@base)
  end

  def test_bare_names_are_safe
    %w[default tux bud-frogs three_eyes v2 dragon.and.cow].each do |name|
      assert CowPath.safe_name?(name), "should accept #{name.inspect}"
    end
  end

  def test_hostile_names_are_unsafe
    ["", "..", "../secret", "..\\secret", "a/b", "a\\b", "/etc/passwd",
      "C:secret", "C:\\Windows\\win", "tux\0", "..%2Fsecret", "%2e%2e/secret", nil].each do |name|
      refute CowPath.safe_name?(name), "should reject #{name.inspect}"
    end
  end

  def test_normal_cow_names_still_load
    assert_includes drawn("tux"), "TUX"
    assert_includes drawn("default"), "DEFAULT"
  end

  def test_unknown_cow_falls_back_to_default
    assert_includes drawn("does-not-exist"), "DEFAULT"
  end

  def test_relative_traversal_falls_back_to_default
    # Sanity: the target really is reachable by naive joining.
    assert File.exist?(File.join(@cows, "..", "secret.cow"))
    ["../secret", "..\\secret", "./../secret", "tux/../../secret"].each do |hostile|
      assert_includes drawn(hostile), "DEFAULT", "for #{hostile.inspect}"
    end
  end

  def test_absolute_path_falls_back_to_default
    assert_includes drawn(File.join(@base, "secret")), "DEFAULT"
  end

  def test_nested_names_are_refused
    ["nested/inner", "nested\\inner"].each do |nested|
      assert_includes drawn(nested), "DEFAULT", "for #{nested.inspect}"
    end
  end

  def test_encoded_and_nul_names_fall_back_to_default
    ["..%2Fsecret", "%2e%2e%2fsecret", "%2E%2E%5Csecret", "tux\0../secret"].each do |hostile|
      assert_includes drawn(hostile), "DEFAULT", "for #{hostile.inspect}"
    end
  end

  # Layer 2 in action: "evil" is syntactically fine, but the file it names is a
  # symlink leading out of the cows directory.
  def test_symlink_escape_falls_back_to_default
    skip "creating symlinks needs elevated privileges on Windows" if Gem.win_platform?

    File.symlink(File.join(@base, "secret.cow"), File.join(@cows, "evil.cow"))
    assert_includes drawn("evil"), "DEFAULT"
  end

  def test_repository_cows_load
    cows = File.expand_path("../../../../specs/cows", __dir__)
    refute_equal File.read(CowPath.resolve("default", cows)),
      File.read(CowPath.resolve("tux", cows))
  end

  private

  def write_cow(path, body)
    FileUtils.mkdir_p(File.dirname(path))
    File.write(path, "$the_cow = <<EOC;\n#{body}\nEOC\n")
  end

  def drawn(cow_name)
    File.read(CowPath.resolve(cow_name, @cows))
  end
end
