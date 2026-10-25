# frozen_string_literal: true

require "rbconfig"
raise ArgumentError, "unknown build option" unless ARGV.empty? || ARGV == ["--windows"]
root = File.expand_path("..", __dir__)
bundler = Gem.bin_path("bundler", "bundle")
# Each dependency owns its directory and Gemfile explicitly. A parent shell's
# current directory/BUNDLE_GEMFILE must not silently select another gem graph.
[File.expand_path("../matrix", root), root].each do |directory|
  Dir.chdir(directory) do
    env = {"BUNDLE_GEMFILE" => File.join(directory, "Gemfile")}
    system(env, RbConfig.ruby, bundler, "install", exception: true)
    lint = ["exec", "standardrb"]
    lint += ["--except", "Layout/EndOfLine"] unless ARGV.empty?
    system(env, RbConfig.ruby, bundler, *lint, exception: true)
    system(env, RbConfig.ruby, bundler, "exec", "rake", exception: true)
  end
end
