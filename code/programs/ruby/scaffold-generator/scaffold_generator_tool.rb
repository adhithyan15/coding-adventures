#!/usr/bin/env ruby
# frozen_string_literal: true

# scaffold_generator_tool.rb -- Command-Line Entry Point
# =====================================================
#
# The program's logic lives in lib/ (CodingAdventures::ScaffoldGenerator);
# this file only makes it runnable:
#
#     ruby code/programs/ruby/scaffold-generator/scaffold_generator_tool.rb \
#       my-package -l ruby --description "A cool package"
#
# It scaffolds into the repository that contains the *current directory*,
# so run it from anywhere inside the checkout you want to change.
#
# Two things have to be on the load path before `main` can run:
#
#   1. CLI Builder and its transitive gems. They are local path gems listed
#      in this package's Gemfile, so we point Bundler at that Gemfile
#      explicitly. Bundler would otherwise search upward from the current
#      directory, which is the caller's, not ours.
#   2. This package's own lib/.

ENV["BUNDLE_GEMFILE"] = File.join(__dir__, "Gemfile")
require "bundler/setup"

$LOAD_PATH.unshift(File.join(__dir__, "lib"))
require "coding_adventures_scaffold_generator"

CodingAdventures::ScaffoldGenerator.main
