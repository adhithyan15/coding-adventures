# frozen_string_literal: true

require "rbconfig"

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # Crash signatures, frame normalisation and the one public failure line
      # (VER06 sections 5.2 and 6, VER06i sections 5 and 7).
      module Signature
        ENGINE_DIR = "#{File.expand_path("..", __dir__)}/".freeze

        # Directories whose code is never "the target": the interpreter's
        # libraries and installed gems.  Computed once.
        def self.library_dirs
          @library_dirs ||= begin
            keys = %w[rubylibdir rubyarchdir sitelibdir sitearchdir vendorlibdir vendorarchdir]
            dirs = keys.filter_map { |k| RbConfig::CONFIG[k] }
            dirs.concat(Gem.path) if defined?(Gem) && Gem.respond_to?(:path)
            dirs.reject(&:empty?).map { |d| "#{File.expand_path(d)}/" }.uniq.freeze
          end
        end

        # Is this source path engine, interpreter or gem plumbing (or a
        # pseudo-path such as "<internal:kernel>" or "(eval at ...)")?
        def self.plumbing?(path)
          return true unless path.is_a?(String)
          return true if path.start_with?("<internal:", "(eval")

          full = File.expand_path(path)
          full.start_with?(ENGINE_DIR) || library_dirs.any? { |d| full.start_with?(d) }
        end

        # The low 32 bits of fnv1a64(kind ":" frames joined by ":"), 8 lowercase hex.
        #
        # Frames are NOT escaped (VER06 section 9.4 item 6): one frame "x:y" and two
        # frames "x", "y" hash alike.  This lane never produces a frame with a colon
        # (normalise_frame), so the ambiguity cannot arise from Ruby targets.
        def self.crash_signature(kind, frames)
          raise TypeError, "kind must be a String" unless kind.is_a?(String)
          unless frames.is_a?(Array) && frames.all?(String)
            raise TypeError, "frames must be an Array of Strings"
          end

          format("%08x", Prng.fnv1a64("#{kind}:#{frames.join(":")}") & 0xFFFF_FFFF)
        end

        def self.duplicate_crash?(kind_a, frames_a, kind_b, frames_b)
          crash_signature(kind_a, frames_a) == crash_signature(kind_b, frames_b)
        end

        NAME_CHARS = /[^A-Za-z0-9._<>$\/#-]/

        # Reduces bytes to the characters a grammar allows; every other BYTE (a
        # control character, a newline, part of a multi-byte character) becomes `_`.
        def self.token(text, limit)
          bytes = text.byteslice(0, limit).b # truncated BEFORE it is encoded
          bytes.gsub(/[^A-Za-z0-9._\/-]/n, "_")
        end

        # The only public failure line: `not ok N - target [failure-id:hex8]`.  It
        # carries no input bytes, seed, message or stack trace (VER00 7.3).
        def self.format_tap_failure(test_num, target, failure_id, sig_hash)
          Limits.check_int!(test_num, "test_num")
          raise ArgumentError, "test_num must be in 1..2147483647" unless test_num.between?(1, 2**31 - 1)
          raise TypeError, "target must be a String" unless target.is_a?(String)
          unless FailureIds.fuzz_failure_id?(failure_id)
            raise ArgumentError, "failure id is not one of the registered fuzz failure ids"
          end
          unless sig_hash.is_a?(String) && sig_hash.bytesize == 8 && sig_hash.match?(/\A[0-9a-f]{8}\z/n)
            raise ArgumentError, "sig hash must be exactly 8 lowercase hex digits"
          end

          "not ok #{test_num} - #{token(target, Limits::MAX_TARGET_LEN)} [#{failure_id}:#{sig_hash}]"
        end

        # A report as public, opaque failure lines (none for a clean one): one per
        # crash signature, then one for timeouts and one for OOMs, whose hash is the
        # signature of the bare kind (they have no stack to hash).
        def self.tap_failures(report, target, first_test = 1)
          lines = report.crashes.map.with_index do |rec, i|
            format_tap_failure(first_test + i, target, FailureIds::CRASH, rec.signature)
          end
          [[report.timeouts, FailureIds::TIMEOUT, "timeout"], [report.ooms, FailureIds::OOM, "oom"]].each do |count, id, kind|
            next unless count.positive?

            lines << format_tap_failure(first_test + lines.size, target, id, crash_signature(kind, []))
          end
          lines
        end

        # A crash kind from a class name: `::` -> `.`, anything outside
        # [A-Za-z0-9._<>$/-] -> `_`, at most 64 characters.
        def self.normalise_kind(name)
          text = name.is_a?(String) ? name : "<anonymous>"
          text.gsub("::", ".").gsub(/[^A-Za-z0-9._<>$\/-]/, "_")[0, Limits::MAX_KIND_LEN]
        end

        # `basename#label`, with every character outside [A-Za-z0-9._<>$/#-] made
        # `_`, so no frame contains `:`.
        def self.normalise_frame(path, label)
          "#{File.basename(path.to_s)}##{label}".gsub(NAME_CHARS, "_")
        end

        # Drops consecutive duplicates (a recursive crash hashes alike at any depth)
        # and keeps the innermost MAX_FRAMES.
        def self.collapse(frames)
          out = []
          frames.each do |f|
            next if out.last == f

            out << f
            break if out.size >= Limits::MAX_FRAMES
          end
          out
        end

        KERNEL_CLASS = Kernel.instance_method(:class)
        MODULE_NAME = Module.instance_method(:name)
        EXC_LOCATIONS = Exception.instance_method(:backtrace_locations)
        LOC_PATH = Thread::Backtrace::Location.instance_method(:path)
        LOC_LABEL = Thread::Backtrace::Location.instance_method(:label)
        LOC_BASE = Thread::Backtrace::Location.instance_method(:base_label)

        # The class name of an exception, read WITHOUT calling any method the
        # exception or its class defines: `Kernel#class` and `Module#name` are bound
        # from their defining modules, so a hostile `def class`, `def self.name` or
        # `def to_s` cannot raise or lie.
        def self.kind_of_exception(error)
          klass = KERNEL_CLASS.bind_call(error)
          normalise_kind(MODULE_NAME.bind_call(klass))
        rescue NoMemoryError, SignalException, SystemExit
          raise
        rescue Exception # rubocop:disable Lint/RescueException
          "<unreadable>"
        end

        # Frames of an exception, innermost first, from `backtrace_locations` bound
        # from Exception (a hostile override cannot intercept it) and fully guarded:
        # nil, a non-array, or ANY error while reading means no frames.  Engine,
        # interpreter and gem frames are dropped, the stack is cut at the executor's
        # own frame, and at most MAX_WALK locations are inspected.
        def self.frames_of(error)
          locations = EXC_LOCATIONS.bind_call(error)
          return [] unless locations.is_a?(Array)

          frames = []
          locations.first(Limits::MAX_WALK).each do |loc|
            path = LOC_PATH.bind_call(loc)
            if path.is_a?(String) && File.expand_path(path).start_with?(ENGINE_DIR)
              break if LOC_BASE.bind_call(loc) == "run_target"

              next
            end
            next if plumbing?(path)

            frames << normalise_frame(path, LOC_LABEL.bind_call(loc))
          end
          collapse(frames)
        rescue NoMemoryError
          raise
        rescue Exception # rubocop:disable Lint/RescueException
          []
        end
      end
    end
  end
end
