# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_immutable_list_native.rb -- Entry point for the gem
# --------------------------------------------------------------------------
#
# This file is the main require target for the gem. It loads:
# 1. The compiled Rust native extension (.so/.bundle/.dll)
# 2. The version constant
#
# The native extension defines:
#   CodingAdventures::ImmutableListNative::ImmutableList
#
# which is a Rust-backed persistent vector using a 32-way trie with
# structural sharing. Every mutation (push, set, pop) returns a new list,
# leaving the original unchanged.

require_relative "coding_adventures/immutable_list_native/version"

# Load the compiled native extension
# Ruby will search for immutable_list_native.so (Linux),
# immutable_list_native.bundle (macOS), or immutable_list_native.dll (Windows)
require "immutable_list_native"
