# frozen_string_literal: true

# Smoke test: require the freshly built extension into a real Ruby and call it.
#
# font-parser-ruby has no Rust unit tests.  It is a thin shim over the
# font-parser crate, and everything that can go wrong in the shim lives at the
# boundary: a Ruby C API symbol that does not link, a hard-coded special
# constant that is wrong for this Ruby, a data pointer read at the wrong
# offset, an exception that is not raised.  Those only show up when an
# interpreter actually loads the compiled library, so that is what this does:
#
#   cargo build --release -p font-parser-ruby --target-dir target
#   ruby tests/smoke_test.rb
#
# On Windows it also proves that build.rs produced a working import library for
# Ruby's DLL.
#
# The library is copied to a temporary directory under the file name Ruby
# expects (`font_parser_native.<DLEXT>`: .so on Linux and Windows, .bundle on
# macOS), because cargo's file name differs per platform.
#
# Plain assertions, no gems: this must run with nothing but `ruby` on PATH.

require "fileutils"
require "rbconfig"
require "tmpdir"

PACKAGE_DIR = File.expand_path("..", __dir__)
FIXTURE = File.expand_path("../../../fixtures/fonts/Inter-Regular.ttf", PACKAGE_DIR)

def built_library
  # The recipe passes `--target-dir target`, deliberately NOT the shared
  # workspace target: font-parser-node, -python and -ruby all emit a cdylib
  # named font_parser_native and would overwrite each other's file.
  target_dir = File.join(PACKAGE_DIR, "target")
  name = case RbConfig::CONFIG["host_os"]
         when /darwin/ then "libfont_parser_native.dylib"
         when /mswin|mingw|cygwin/ then "font_parser_native.dll"
         else "libfont_parser_native.so"
         end
  built = File.join(target_dir, "release", name)
  abort "built extension not found at #{built}; run the cargo build first" unless File.exist?(built)
  built
end

staging = Dir.mktmpdir("font-parser-ruby-")
extension = File.join(staging, "font_parser_native.#{RbConfig::CONFIG["DLEXT"]}")
FileUtils.cp(built_library, extension)
require extension

FP = CodingAdventures::FontParserNative
BYTES = File.binread(FIXTURE)

$failures = []
$count = 0

def check(name)
  $count += 1
  yield
  puts "ok   #{name}"
rescue Exception => e # rubocop:disable Lint/RescueException
  $failures << name
  puts "FAIL #{name}: #{e.class}: #{e.message}"
end

def expect_equal(expected, actual)
  raise "expected #{expected.inspect}, got #{actual.inspect}" unless expected == actual
end

def expect_raises(klass, pattern)
  yield
  raise "expected #{klass} to be raised"
rescue klass => e
  raise "message #{e.message.inspect} does not match #{pattern.inspect}" unless pattern.match?(e.message)
end

check("exposes the documented surface") do
  %i[load font_metrics glyph_id glyph_metrics kerning].each do |name|
    raise "missing #{name}" unless FP.respond_to?(name)
  end
end

check("font_metrics reads Inter") do
  font = FP.load(BYTES)
  expect_equal(
    {units_per_em: 2048, ascender: 1984, descender: -494, line_gap: 0, x_height: 1118, cap_height: 1490,
     num_glyphs: 2926, family_name: "Inter", subfamily_name: "Regular"},
    FP.font_metrics(font)
  )
end

check("glyph_id and glyph_metrics follow cmap and hmtx") do
  font = FP.load(BYTES)
  a = FP.glyph_id(font, 0x41)
  expect_equal(2, a)
  expect_equal({advance_width: 1413, left_side_bearing: 52}, FP.glyph_metrics(font, a))
end

check("a missing glyph is the real nil, not a stale special constant") do
  font = FP.load(BYTES)
  # U+10FFFF has no glyph.  Ruby 3.2 changed Qnil from 0x08 to 0x04, so a
  # hard-coded constant here would come back as a bogus object, not nil.
  raise "expected nil, got #{FP.glyph_id(font, 0x10FFFF).inspect}" unless FP.glyph_id(font, 0x10FFFF).nil?
end

check("kerning is zero for Inter (GPOS, not the legacy kern table)") do
  font = FP.load(BYTES)
  expect_equal(0, FP.kerning(font, FP.glyph_id(font, 0x41), FP.glyph_id(font, 0x56)))
end

check("a FontFile survives garbage collection") do
  font = FP.load(BYTES)
  GC.start
  expect_equal("Inter", FP.font_metrics(font)[:family_name])
end

check("bad input raises instead of crashing the process") do
  expect_raises(RuntimeError, /buffer too short/) { FP.load("junk".b) }
  expect_raises(RuntimeError, /expected a FontFile/) { FP.font_metrics(Object.new) }
  expect_raises(TypeError, /allocator undefined/) { FP::FontFile.new }
end

puts "#{$count - $failures.size}/#{$count} passed"
exit($failures.empty? ? 0 : 1)
