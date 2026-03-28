# frozen_string_literal: true

require_relative "coding_adventures/garbage_collector/version"
require_relative "coding_adventures/garbage_collector/heap_object"
require_relative "coding_adventures/garbage_collector/mark_sweep"

# CodingAdventures::GarbageCollector provides mark-and-sweep GC for
# heap-allocated Lisp objects (ConsCell, LispSymbol, LispClosure).
#
# @example
#   gc = CodingAdventures::GarbageCollector::MarkAndSweepGC.new
#   addr = gc.allocate(CodingAdventures::GarbageCollector::ConsCell.new(car: 42))
#   gc.collect(roots: [addr])
module CodingAdventures
  module GarbageCollector
  end
end
