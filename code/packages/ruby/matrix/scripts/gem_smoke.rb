# frozen_string_literal: true

require "json"
require "open3"
require "rbconfig"
require "rubygems/package"
require "rubygems/installer"
require "tmpdir"

# Build from this fixed source, never a caller-provided gem or executable.
# The temporary directory owns all artifacts and is removed on success/error.
root = File.expand_path("..", __dir__)
Dir.chdir(root) do
  spec = Gem::Specification.load("coding_adventures_matrix.gemspec")
  raise "invalid matrix gem metadata" unless spec && spec.name == "coding_adventures_matrix" && spec.version.to_s == "0.2.0"
  raise "unexpected runtime dependency" unless spec.runtime_dependencies.empty?

  Dir.mktmpdir("nn44-matrix-") do |temporary|
    artifact = File.join(temporary, "matrix.gem")
    Gem::Package.build(spec, false, true, artifact)
    home = File.join(temporary, "gems")
    installed = Gem::Installer.at(artifact, install_dir: home, document: [], ignore_dependencies: true).install
    raise "wrong installed gem" unless installed.full_name == "coding_adventures_matrix-0.2.0"

    # Remove Bundler's injected load paths/options. The child can see only its
    # isolated installed gem and Ruby's standard library, not this checkout.
    env = {
      "GEM_HOME" => home, "GEM_PATH" => home,
      "RUBYOPT" => nil, "RUBYLIB" => nil, "RUBYGEMS_GEMDEPS" => nil, "BUNDLE_GEMFILE" => nil,
      "BUNDLE_BIN_PATH" => nil
    }
    oracle = <<~RUBY
      require "json"
      require "matrix_ml"
      expected = [[19.0, 22.0], [43.0, 50.0]]
      actual = Matrix.new([[1.0, 2.0], [3.0, 4.0]]).dot(Matrix.new([[5.0, 6.0], [7.0, 8.0]])).data
      abort "installed matrix dot mismatch" unless actual == expected
      loaded = $LOADED_FEATURES.find { |path| path.end_with?("/matrix_ml.rb") }
      wanted = File.join(Gem.dir, "gems", "coding_adventures_matrix-0.2.0", "lib", "matrix_ml.rb")
      abort "checkout leaked into installed smoke" unless loaded && File.realpath(loaded) == File.realpath(wanted)
      puts JSON.generate(contract: "NN44", native_library: "ruby/matrix", dot: actual, neural_training: false)
    RUBY
    output, errors, status = Open3.capture3(env, RbConfig.ruby, "-e", oracle, chdir: temporary)
    raise "installed gem smoke failed: #{errors}" unless status.success?
    receipt = JSON.parse(output)
    raise "wrong smoke receipt" unless receipt.fetch("contract") == "NN44" && receipt.fetch("neural_training") == false
    puts output
  end
end
