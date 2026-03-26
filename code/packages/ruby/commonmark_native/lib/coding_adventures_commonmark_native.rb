# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_commonmark_native.rb — Entry point for the gem
# --------------------------------------------------------------------------
#
# This file is the main require target for the gem. It loads:
# 1. The version constant
# 2. The compiled Rust native extension (.so/.bundle/.dll)
#
# The native extension defines:
#   CodingAdventures::CommonmarkNative.markdown_to_html(markdown)
#   CodingAdventures::CommonmarkNative.markdown_to_html_safe(markdown)
#
# which are Rust-backed functions that parse CommonMark Markdown and render
# it to HTML using our zero-dependency `commonmark` Rust crate.

require_relative "coding_adventures/commonmark_native/version"

# Load the compiled native extension.
# Ruby will search for commonmark_native.so (Linux),
# commonmark_native.bundle (macOS), or commonmark_native.dll (Windows).
require "commonmark_native"
