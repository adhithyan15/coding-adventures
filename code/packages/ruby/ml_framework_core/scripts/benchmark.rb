# frozen_string_literal: true

# scripts/benchmark.rb — performance characterization for ml_framework_core
# ===========================================================================
#
# Runs the same 2-layer MLP from test/end_to_end_training_test.rb at
# increasing batch sizes and prints a markdown table of forward +
# backward timings.
#
# Two intended uses:
#
#   1. **Find the Ruby-vs-Rust crossover** — at small batch sizes the
#      pure-Ruby fallback wins because the JSON+hex+FFI dispatch
#      overhead dominates.  As batches grow, Rust's f32 SIMD pulls
#      ahead.  The table makes the crossover visible at a glance.
#
#   2. **Spot regressions** — re-run after any change to ops.rb /
#      autograd.rb and compare with prior runs.  A 2-3x slowdown
#      anywhere should be investigated.
#
# Usage:
#
#   cd code/packages/ruby/ml_framework_core
#   ruby -Ilib scripts/benchmark.rb
#
# No CLI arguments.  No file output.  Stdout-only.

require "coding_adventures/ml_framework_core"

T = CodingAdventures::MLFrameworkCore::Tensor

# Configurable batch sizes.  Picked to bracket the 10_000-cell dispatch
# threshold from both sides:
#   100   →  100 cells   →  pure Ruby
#   1000  → 1000 cells   →  pure Ruby
#   5000  → 5000 cells   →  pure Ruby (still below 10k)
#   10000 → 10000 cells  →  AT the threshold
#   50000 → 50000 cells  →  WAY above the threshold (Rust dispatch)
BATCH_SIZES = [100, 1000, 5000, 10_000, 50_000].freeze
WARMUP_RUNS = 2
TIMED_RUNS  = 5

# Lazy-detect whether the Rust dispatch path is available.  If the
# `matrix_rust_ruby` gem's native extension isn't built, the lazy
# require inside Ops.run_envelope will raise LoadError — we surface
# that as a header note rather than crashing.
def rust_available?
  require "coding_adventures/matrix_rust_ruby"
  true
rescue LoadError
  false
end

# A single forward + backward pass at the given batch size.
# Returns [forward_seconds, backward_seconds].
def time_step(batch_size)
  # Build inputs: x is (batch, 1), target is (batch, 1).  The data
  # values themselves don't matter — we're measuring time, not loss.
  x_data = Array.new(batch_size) { |i| [i.to_f / batch_size] }
  y_data = Array.new(batch_size) { |i| [2.0 * (i.to_f / batch_size) + 3.0] }
  x = T.new(x_data)
  target = T.new(y_data)

  # 1-hidden-unit MLP — minimal layer count to focus on per-cell cost
  # rather than per-op orchestration overhead.
  w1 = T.new([[0.5, -0.3]]); w1.requires_grad = true
  w2 = T.new([[0.4], [0.7]]); w2.requires_grad = true

  # --- forward ---
  fwd_start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  pred = x.matmul(w1).relu.matmul(w2)
  diff = pred - target
  loss = (diff * diff).mean
  fwd_end = Process.clock_gettime(Process::CLOCK_MONOTONIC)

  # --- backward ---
  bwd_start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  loss.backward
  bwd_end = Process.clock_gettime(Process::CLOCK_MONOTONIC)

  [fwd_end - fwd_start, bwd_end - bwd_start]
end

# Pick the dispatch label for a given batch size.  Heuristic: any
# tensor that crosses the threshold somewhere in the forward graph
# (matmul, relu, sub, mul, mean) will exercise the Rust path.  The
# (batch × hidden) intermediate of `x.matmul(w1).relu` is batch * 2,
# so 5000 batch → 10000-cell intermediate → Rust kicks in.  Crude
# but useful enough as a column label.
def dispatch_label(batch_size, rust_available)
  return "Ruby (no Rust)" unless rust_available
  return "Ruby"           if batch_size < 5_000
  "Rust"
end

def run_benchmark
  rust_ok = rust_available?

  puts "# ml_framework_core benchmark"
  puts
  puts "Forward + backward pass on a 2-layer MLP (1 input → 2 hidden ReLU → 1 output)"
  puts "at increasing batch sizes.  Each timing is the median of #{TIMED_RUNS} runs"
  puts "after #{WARMUP_RUNS} warmup runs."
  puts
  puts "- Ruby version:    #{RUBY_VERSION} (#{RUBY_PLATFORM})"
  puts "- matrix_rust_ruby: #{rust_ok ? "available" : "NOT BUILT — Ruby fallback only"}"
  puts
  puts "| batch  | forward (ms) | backward (ms) | total (ms) | dispatch       |"
  puts "|--------|--------------|---------------|------------|----------------|"

  results = []
  BATCH_SIZES.each do |batch_size|
    # If matrix_rust_ruby isn't available AND this batch would trigger
    # a Rust dispatch (intermediate matmul shape ≥ 10k cells), skip
    # the row rather than crashing with LoadError.  The dispatch is
    # triggered when batch * hidden_dim ≥ DISPATCH_THRESHOLD; here
    # hidden_dim = 2, so batch ≥ 5000 will route through Rust.
    if !rust_ok && batch_size >= 5_000
      printf("| %6d | %12s | %13s | %10s | %-14s |\n",
             batch_size, "(skipped)", "(skipped)", "(skipped)", "Rust needed")
      results << [batch_size, nil, nil, nil, "Rust needed"]
      next
    end

    # Warmup runs to let any lazy requires (matrix_rust_ruby) load and
    # to give Ruby's GC a chance to settle.
    WARMUP_RUNS.times { time_step(batch_size) }

    # Timed runs.  We collect all and take the median for stability —
    # the mean is dominated by the worst-case outliers from GC pauses.
    samples = Array.new(TIMED_RUNS) { time_step(batch_size) }
    fwd_ms = samples.map { |f, _| f * 1000.0 }.sort[TIMED_RUNS / 2]
    bwd_ms = samples.map { |_, b| b * 1000.0 }.sort[TIMED_RUNS / 2]
    total_ms = fwd_ms + bwd_ms
    dispatch = dispatch_label(batch_size, rust_ok)

    printf("| %6d | %12.2f | %13.2f | %10.2f | %-14s |\n",
           batch_size, fwd_ms, bwd_ms, total_ms, dispatch)
    results << [batch_size, fwd_ms, bwd_ms, total_ms, dispatch]
  end

  puts
  puts "## Notes"
  puts
  if rust_ok
    # Find the smallest batch where Rust dispatch kicked in.
    rust_threshold = results.find { |_, _, _, _, d| d == "Rust" }&.first
    if rust_threshold
      puts "- Rust dispatch crossed in around batch=#{rust_threshold} cells per intermediate."
      puts "  Below that, the pure-Ruby element-wise loop is faster than the"
      puts "  JSON+hex+FFI envelope round-trip."
    else
      puts "- All batches stayed in the pure-Ruby path."
    end
  else
    puts "- All timings reflect the pure-Ruby fallback path."
    puts "  Build the matrix_rust_ruby native ext (`cd ../matrix_rust_ruby && bundle exec rake compile`)"
    puts "  to compare against Rust dispatch."
  end
  puts
  puts "## Reproducing"
  puts
  puts "    cd code/packages/ruby/ml_framework_core"
  puts "    ruby -Ilib scripts/benchmark.rb"
end

run_benchmark
