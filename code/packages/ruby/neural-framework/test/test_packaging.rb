# frozen_string_literal: true

require_relative "test_helper"

class PackagingTest < Minitest::Test
  def test_metadata_and_leaf_first_build_fronts
    root = File.expand_path("..", __dir__)
    spec = Gem::Specification.load(File.join(root, "coding_adventures_neural_framework.gemspec"))
    assert_equal "coding_adventures_neural_framework", spec.name
    assert_equal "0.1.0", spec.version.to_s
    assert_equal "MIT", spec.license
    assert_empty spec.extensions
    assert_equal ["coding_adventures_matrix"], spec.runtime_dependencies.map(&:name)
    assert_equal "~> 0.2.0", spec.runtime_dependencies.first.requirement.to_s
    assert_equal %w[lib/neural_framework.rb examples/xor.rb README.md CHANGELOG.md LICENSE required_capabilities.json].sort, spec.files.sort
    spec.files.each { |file| assert File.file?(File.join(root, file)), file }
    common = ["python -m unittest discover -s ../../../scripts/tests -p test_neural_framework_ruby.py", "python ../../../scripts/validate_neural_framework_ruby.py"]
    assert_equal ["ruby scripts/build.rb", *common], File.readlines(File.join(root, "BUILD"), chomp: true)
    assert_equal ["ruby scripts/build.rb --windows", *common], File.readlines(File.join(root, "BUILD_windows"), chomp: true)
    build = File.read(File.join(root, "scripts/build.rb"))
    assert_includes build, '[File.expand_path("../matrix", root), root]'
    assert_includes build, "Dir.chdir(directory) do"
    assert_includes build, '"BUNDLE_GEMFILE" => File.join(directory, "Gemfile")'
    assert_includes build, 'lint += ["--except", "Layout/EndOfLine"] unless ARGV.empty?'
    manifest = JSON.parse(File.read(File.join(root, "required_capabilities.json")))
    assert_equal "ruby/neural-framework", manifest["package"]
    assert_empty manifest["capabilities"]
  end
end
