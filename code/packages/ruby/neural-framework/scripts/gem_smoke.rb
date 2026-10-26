# frozen_string_literal: true

require "json"
require "open3"
require "rbconfig"
require "rubygems/package"
require "rubygems/installer"
require "tmpdir"

root = File.expand_path("..", __dir__)
Dir.mktmpdir("nn45-native-") do |temporary|
  home = File.join(temporary, "gems")
  [[File.expand_path("../matrix", root), "coding_adventures_matrix", "0.2.0"], [root, "coding_adventures_neural_framework", "0.1.0"]].each do |directory, name, version|
    Dir.chdir(directory) do
      spec = Gem::Specification.load("#{name}.gemspec")
      raise "invalid gem metadata" unless spec && spec.name == name && spec.version.to_s == version
      raise "missing explicit payload" unless spec.files.all? { |file| File.file?(file) }
      dependencies = spec.runtime_dependencies.map(&:name)
      raise "unexpected runtime dependency" unless dependencies == ((name == "coding_adventures_matrix") ? [] : ["coding_adventures_matrix"])
      artifact = File.join(temporary, "#{name}.gem")
      Gem::Package.build(spec, false, true, artifact)
      Gem::Installer.at(artifact, install_dir: home, document: [], ignore_dependencies: true).install
    end
  end
  env = {"GEM_HOME" => home, "GEM_PATH" => home, "RUBYOPT" => nil, "RUBYLIB" => nil, "RUBYGEMS_GEMDEPS" => nil, "BUNDLE_GEMFILE" => nil, "BUNDLE_BIN_PATH" => nil}
  oracle = <<~RUBY
    require "json"
    require "neural_framework"
    {"matrix_ml.rb" => "coding_adventures_matrix-0.2.0", "neural_framework.rb" => "coding_adventures_neural_framework-0.1.0"}.each do |file, gem_name|
      loaded = $LOADED_FEATURES.find { |path| path.end_with?("/" + file) }
      wanted = File.join(Gem.dir, "gems", gem_name, "lib", file)
      abort "checkout leaked into installed smoke" unless loaded && File.realpath(loaded) == File.realpath(wanted)
    end
    nf = CodingAdventures::NeuralFramework
    model = nf::DenseNetwork.new([nf::DenseLayer.new(weights: [[2], [3]], biases: [1], activation: "linear")])
    abort "prediction mismatch" unless model.predict([[4, 5]]) == [[24.0]]
    model.train_step([[4, 5]], [[0]], 0.001)
    loaded = nf::DenseNetwork.from_json(model.to_json)
    abort "roundtrip mismatch" unless loaded.to_checkpoint == model.to_checkpoint && loaded.step == 1
    puts JSON.generate(contract: "NN45", native_lane: "ruby", installed_training: true, five_language_interchange: "pending-separate-tranche")
  RUBY
  output, errors, status = Open3.capture3(env, RbConfig.ruby, "-e", oracle, chdir: temporary)
  raise "installed gem smoke failed: #{errors}" unless status.success?
  receipt = JSON.parse(output)
  raise "wrong installed receipt" unless receipt.fetch("contract") == "NN45" && receipt.fetch("installed_training") == true
  puts output
end
