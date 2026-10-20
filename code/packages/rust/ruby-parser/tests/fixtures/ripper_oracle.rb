# Regenerate the Ripper.lex oracle files for the R0 differential corpus.
#
#   ruby code/packages/rust/ruby-parser/tests/fixtures/ripper_oracle.rb
#
# Each r0/<name>.rb gets r0/<name>.ripper: one line per Ripper.lex token,
# `line<TAB>column<TAB>event<TAB>value`, with `\`, newline and tab in the
# value written as `\\`, `\n` and `\t`. Whitespace (`on_sp`) is dropped.
# The first line records the interpreter that produced the file; LANG84
# "Corpora and oracles" checks it only against that release's pair.
require "ripper"

dir = File.join(__dir__, "r0")
Dir.glob(File.join(dir, "*.rb")).sort.each do |path|
  source = File.read(path)
  lines = ["# oracle: Ripper.lex, ruby #{RUBY_VERSION} (#{RUBY_RELEASE_DATE} revision #{RUBY_REVISION[0, 10]})"]
  Ripper.lex(source).each do |(line, column), event, token, _state|
    next if event == :on_sp
    escaped = token.gsub("\\") { "\\\\" }.gsub("\n", "\\n").gsub("\t", "\\t")
    lines << [line, column, event, escaped].join("\t")
  end
  File.write(path.sub(/\.rb\z/, ".ripper"), lines.join("\n") + "\n")
end
