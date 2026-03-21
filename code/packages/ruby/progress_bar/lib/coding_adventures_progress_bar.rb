# frozen_string_literal: true

# ==========================================================================
# ProgressBar -- A Thread-Safe Text-Based Progress Bar
# ==========================================================================
#
# This gem is the Ruby port of the Go progress-bar package. It provides a
# reusable, thread-safe progress bar that renders to any IO stream, showing
# completed/total count, in-flight item names, and elapsed time.
#
# Two tracker types are available:
#
# 1. Tracker   -- the real implementation, backed by Thread::Queue and a
#                 background rendering thread.
#
# 2. NullTracker -- a no-op stand-in with the same interface. Use this when
#                   you want to disable progress display without changing
#                   calling code.
#
# Both support flat mode (one level) and hierarchical mode (parent/child).
# ==========================================================================

require_relative "coding_adventures/progress_bar/version"
require_relative "coding_adventures/progress_bar/tracker"

module CodingAdventures
  module ProgressBar
  end
end
