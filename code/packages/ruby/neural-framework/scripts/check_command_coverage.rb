# frozen_string_literal: true

require "coverage"
require "stringio"
Coverage.start(lines: true)
require_relative "../examples/xor"
# Exercise the actual executable error branch without spawning an unmeasured
# process. Restore process globals even when its intended exit is raised.
path = File.expand_path("../examples/xor.rb", __dir__)
saved_name = $PROGRAM_NAME
saved_args = ARGV.dup
saved_error = $stderr
begin
  $PROGRAM_NAME = path
  ARGV.replace(["--invalid"])
  $stderr = StringIO.new
  load path
rescue SystemExit => error
  raise "wrong command exit" unless error.status == 1
ensure
  $PROGRAM_NAME = saved_name
  ARGV.replace(saved_args)
  $stderr = saved_error
end
# load replaces the file's coverage counters: exercise methods after the final
# load so the report measures this same instrumented version, not its predecessor.
example = CodingAdventures::NeuralFramework::XORExample
output = StringIO.new
example.run([], StringIO.new, output)
checkpoint = JSON.parse(output.string).fetch("checkpoint")
example.run(["--resume"], StringIO.new(JSON.generate(checkpoint)), StringIO.new)
lines = Coverage.result.fetch(path).fetch(:lines).compact
percent = 100.0 * lines.count { |hits| hits > 0 } / lines.length
raise "command coverage below 80%: #{percent}" unless percent >= 80
puts "Native XOR command line coverage: #{percent.round(2)}%"
