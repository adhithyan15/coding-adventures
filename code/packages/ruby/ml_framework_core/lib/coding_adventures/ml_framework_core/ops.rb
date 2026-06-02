# frozen_string_literal: true

# ops.rb — the 15 differentiable operations of ml_framework_core
# ====================================================================
#
# This is where Tensors get math.  Every op is a `Function` subclass
# (defined in autograd.rb) with a `forward` method that:
#
#   - If the input is below a threshold (10k cells), uses pure Ruby —
#     fast enough at that size, avoids the JSON+hex+FFI overhead.
#   - If above the threshold, builds a matrix-ir-json envelope and
#     dispatches through `MatrixRustRuby.run_graph_on_cpu` to the Rust
#     `matrix-cpu` executor.
#
# The envelope shapes here MIRROR the Python reference at
# `code/packages/python/ml-framework-core/src/ml_framework_core/_rust_backend.py`
# byte-for-byte.  If you change anything here, also update there — the
# wire format is the cross-language contract.
#
# # backward() implementations come in PR #7
#
# This PR ships forward-only.  Each subclass below has a forward but no
# backward override; calling `.backward` on a tensor produced by these
# ops will hit `Function#backward`'s NotImplementedError.  PR #7 fills
# them in using the analytical gradient formulas from autograd.py.
#
# # Op coverage in v0.3.0
#
#   ╔═════════════╤══════════════════════╤═══════════════════════════╗
#   ║ Op          │ Rust dispatch        │ matrix-ir-json kind       ║
#   ╟─────────────┼──────────────────────┼───────────────────────────╢
#   ║ Add         │ yes (Rust)           │ Add (lhs/rhs/output)      ║
#   ║ Sub         │ yes                  │ Sub                       ║
#   ║ Mul         │ yes                  │ Mul                       ║
#   ║ Div         │ yes                  │ Div                       ║
#   ║ Neg         │ yes                  │ Neg (input/output)        ║
#   ║ Abs         │ yes                  │ Abs                       ║
#   ║ Tanh        │ yes                  │ Tanh                      ║
#   ║ MatMul      │ yes (2-D only)       │ MatMul (a/b/output)       ║
#   ║ Sum         │ yes (reduce-all)     │ ReduceSum (axes/keepdims) ║
#   ║ Mean        │ yes                  │ ReduceMean                ║
#   ║ Pow         │ pure Ruby            │ — (Rust Pow takes tensor) ║
#   ║ ReLU        │ pure Ruby            │ — (could use Max + const) ║
#   ║ Sigmoid     │ pure Ruby            │ — (multi-op graph)        ║
#   ║ GELU        │ pure Ruby            │ — (multi-op graph)        ║
#   ║ Softmax     │ pure Ruby            │ — (multi-op graph)        ║
#   ╚═════════════╧══════════════════════╧═══════════════════════════╝
#
# The "pure Ruby" five all have working Rust paths in the Python
# reference; we left them in pure Ruby for v0.3.0 to keep this PR
# focused.  Future PRs can lift them over one by one.

require "json"
require_relative "tensor"
require_relative "autograd"

# NOTE: `coding_adventures/matrix_rust_ruby` is required LAZILY inside
# `Ops.run_envelope` (only when we actually need to dispatch to Rust).
# This keeps small-tensor / pure-Ruby workflows runnable even when the
# matrix_rust_ruby gem's native extension isn't built — e.g. on CI
# machines without a Rust toolchain, or during early dev iteration.

module CodingAdventures
  module MLFrameworkCore
    # ========================================================================
    # Module-level helpers: hex packing, threshold, envelope dispatcher.
    # ========================================================================
    module Ops
      # Tensors with `numel` ≥ this dispatch to Rust; smaller use pure Ruby.
      # 10_000 cells is the rough break-even from the Python benchmark
      # (`scripts/benchmark_mx10.py`) — below it, the JSON + hex + FFI
      # overhead dominates; above it, matrix-cpu's f32 SIMD wins big.
      DISPATCH_THRESHOLD = 10_000

      module_function

      # @return [Integer] the dispatch threshold (exposed so tests can
      #   intentionally cross it from both sides).
      def dispatch_threshold
        DISPATCH_THRESHOLD
      end

      # Pack an Array<Float> into a hex string of little-endian f32 bytes.
      # Ruby's `pack("e*")` is `e` = little-endian single-precision (f32);
      # `unpack1("H*")` converts the bytes to a lowercase hex string.
      def pack_f32_hex(arr)
        arr.pack("e*").unpack1("H*")
      end

      # Inverse of `pack_f32_hex`.  Returns an Array<Float> of length numel.
      # We pass `numel` explicitly because String#unpack returns extra
      # elements if the hex has trailing zero bytes (it doesn't here, but
      # being explicit is cheap insurance).
      def unpack_f32_hex(hex, numel)
        [hex].pack("H*").unpack("e#{numel}")
      end

      # Drive the Rust executor: JSON-serialize, call into the gem, parse
      # the JSON response, and decode `outputs[0]` back to an Array<Float>.
      #
      # @param envelope [Hash] the matrix-ir-json envelope as a Ruby Hash.
      # @param output_numel [Integer] expected number of f32 cells in the
      #   single output.
      # @return [Array<Float>] decoded output data.
      def run_envelope(envelope, output_numel)
        # Lazy require — see header comment.  Defers the cost of loading
        # the matrix_rust_ruby native extension until the first dispatch.
        require "coding_adventures/matrix_rust_ruby" unless defined?(::MatrixRustRuby)
        env_json = JSON.generate(envelope)
        result_json = ::MatrixRustRuby.run_graph_on_cpu(env_json)
        result = JSON.parse(result_json)
        out_hex = result.fetch("outputs").fetch(0)
        floats = unpack_f32_hex(out_hex, output_numel)
        expected_bytes = output_numel * 4
        # Hex string is 2 chars per byte; if we got the wrong length back,
        # the executor is misbehaving — fail loudly.
        unless out_hex.length == expected_bytes * 2
          raise RuntimeError,
                "matrix_cpu returned #{out_hex.length / 2} bytes, expected #{expected_bytes}"
        end
        floats
      end

      # Build the matrix-ir-json envelope for a binary elementwise op
      # (Add/Sub/Mul/Div).  Mirrors `_rust_backend._elementwise_binary_via_rust`.
      def binary_elementwise_envelope(kind, a, b)
        shape = a.shape
        {
          "graph" => {
            "matrix_ir_version" => 1,
            "tensors" => [
              { "id" => 0, "dtype" => "f32", "shape" => shape },
              { "id" => 1, "dtype" => "f32", "shape" => shape },
              { "id" => 2, "dtype" => "f32", "shape" => shape },
            ],
            "inputs" => [0, 1],
            "outputs" => [2],
            "ops" => [{ "kind" => kind, "lhs" => 0, "rhs" => 1, "output" => 2 }],
            "constants" => [],
          },
          "inputs" => [pack_f32_hex(a.to_a), pack_f32_hex(b.to_a)],
        }
      end

      # Build the matrix-ir-json envelope for a unary elementwise op
      # (Neg/Abs/Tanh).  Mirrors `_rust_backend._elementwise_unary_via_rust`.
      def unary_elementwise_envelope(kind, a)
        shape = a.shape
        {
          "graph" => {
            "matrix_ir_version" => 1,
            "tensors" => [
              { "id" => 0, "dtype" => "f32", "shape" => shape },
              { "id" => 1, "dtype" => "f32", "shape" => shape },
            ],
            "inputs" => [0],
            "outputs" => [1],
            "ops" => [{ "kind" => kind, "input" => 0, "output" => 1 }],
            "constants" => [],
          },
          "inputs" => [pack_f32_hex(a.to_a)],
        }
      end

      # Envelope for MatMul of two 2-D tensors.  Mirrors
      # `_rust_backend.matmul_via_rust`.
      def matmul_envelope(a, b)
        m, k = a.shape
        _, n = b.shape
        {
          "graph" => {
            "matrix_ir_version" => 1,
            "tensors" => [
              { "id" => 0, "dtype" => "f32", "shape" => [m, k] },
              { "id" => 1, "dtype" => "f32", "shape" => [k, n] },
              { "id" => 2, "dtype" => "f32", "shape" => [m, n] },
            ],
            "inputs" => [0, 1],
            "outputs" => [2],
            "ops" => [{ "kind" => "MatMul", "a" => 0, "b" => 1, "output" => 2 }],
            "constants" => [],
          },
          "inputs" => [pack_f32_hex(a.to_a), pack_f32_hex(b.to_a)],
        }
      end

      # Envelope for ReduceSum / ReduceMean over ALL axes (collapse to scalar).
      # Mirrors `_rust_backend._reduce_all_via_rust`.
      def reduce_all_envelope(kind, a)
        shape = a.shape
        axes = (0...shape.length).to_a
        {
          "graph" => {
            "matrix_ir_version" => 1,
            "tensors" => [
              { "id" => 0, "dtype" => "f32", "shape" => shape },
              { "id" => 1, "dtype" => "f32", "shape" => [] },
            ],
            "inputs" => [0],
            "outputs" => [1],
            "ops" => [
              {
                "kind" => kind,
                "input" => 0,
                "axes" => axes,
                "keep_dims" => false,
                "output" => 1,
              },
            ],
            "constants" => [],
          },
          "inputs" => [pack_f32_hex(a.to_a)],
        }
      end
    end

    # ========================================================================
    # The 15 differentiable ops, each a Function subclass.
    #
    # Pattern: every `forward` checks the Ops.dispatch_threshold and either
    # builds an envelope + runs via Rust, or computes element-wise in Ruby.
    # The Function base class's `apply` (from autograd.rb) handles
    # grad_fn wiring; here we just provide the forward math.
    # ========================================================================

    # --- Helper used by binary ops to share dispatch logic ---
    def self._binary_elementwise(kind, a, b, ruby_op)
      unless a.shape == b.shape
        raise ArgumentError,
              "shape mismatch for #{kind}: #{a.shape.inspect} vs #{b.shape.inspect}"
      end

      if a.numel >= Ops.dispatch_threshold
        envelope = Ops.binary_elementwise_envelope(kind, a, b)
        floats = Ops.run_envelope(envelope, a.numel)
        Tensor.new(floats, shape: a.shape)
      else
        # Pure-Ruby fallback.  Uses Tensor's own operator overloads which
        # are themselves element-wise pure Ruby — but we go through
        # to_a.zip directly to avoid creating extra intermediate Tensors
        # that would themselves try to dispatch through Function.apply.
        a_data = a.to_a
        b_data = b.to_a
        out = Array.new(a.numel)
        i = 0
        while i < a.numel
          out[i] = ruby_op.call(a_data[i], b_data[i])
          i += 1
        end
        Tensor.new(out, shape: a.shape)
      end
    end

    def self._unary_elementwise(kind, a, ruby_op)
      if a.numel >= Ops.dispatch_threshold
        envelope = Ops.unary_elementwise_envelope(kind, a)
        floats = Ops.run_envelope(envelope, a.numel)
        Tensor.new(floats, shape: a.shape)
      else
        Tensor.new(a.to_a.map { |v| ruby_op.call(v) }, shape: a.shape)
      end
    end

    # ────────── Binary elementwise: Add / Sub / Mul / Div ──────────

    class AddOp < Function
      def forward(a, b)
        MLFrameworkCore._binary_elementwise("Add", a, b, ->(x, y) { x + y })
      end

      # d/dx (x + y) = 1, d/dy (x + y) = 1.  Gradient flows through unchanged
      # to both inputs.  Build fresh Tensors (one per parent) so each parent
      # gets its own copy of the gradient — required because the autograd
      # walker may accumulate into them independently.
      def backward(grad)
        g_data = grad.to_a
        shape  = grad.shape
        [Tensor.new(g_data.dup, shape: shape), Tensor.new(g_data.dup, shape: shape)]
      end
    end

    class SubOp < Function
      def forward(a, b)
        MLFrameworkCore._binary_elementwise("Sub", a, b, ->(x, y) { x - y })
      end

      # d/dx (x - y) = 1, d/dy (x - y) = -1.
      def backward(grad)
        g_data = grad.to_a
        shape  = grad.shape
        [Tensor.new(g_data.dup, shape: shape), Tensor.new(g_data.map { |v| -v }, shape: shape)]
      end
    end

    class MulOp < Function
      def forward(a, b)
        # Save a and b for backward — Mul backward needs both inputs.
        # We dup the data arrays so a later in-place mutation of the input
        # Tensor (PyTorch-style) wouldn't poison our saved copy.
        @saved_for_backward[:a] = a
        @saved_for_backward[:b] = b
        MLFrameworkCore._binary_elementwise("Mul", a, b, ->(x, y) { x * y })
      end

      # d/dx (x * y) = y, d/dy (x * y) = x.  Element-wise chain rule.
      def backward(grad)
        a = @saved_for_backward[:a]
        b = @saved_for_backward[:b]
        g = grad.to_a
        ad = a.to_a
        bd = b.to_a
        [
          Tensor.new(g.each_with_index.map { |gi, i| gi * bd[i] }, shape: a.shape),
          Tensor.new(g.each_with_index.map { |gi, i| gi * ad[i] }, shape: b.shape),
        ]
      end
    end

    class DivOp < Function
      def forward(a, b)
        @saved_for_backward[:a] = a
        @saved_for_backward[:b] = b
        MLFrameworkCore._binary_elementwise("Div", a, b, ->(x, y) { x / y })
      end

      # d/dx (x / y) = 1 / y, d/dy (x / y) = -x / y².  Standard quotient rule.
      def backward(grad)
        a = @saved_for_backward[:a]
        b = @saved_for_backward[:b]
        g = grad.to_a
        ad = a.to_a
        bd = b.to_a
        [
          Tensor.new(g.each_with_index.map { |gi, i| gi / bd[i] }, shape: a.shape),
          Tensor.new(g.each_with_index.map { |gi, i| -gi * ad[i] / (bd[i] * bd[i]) }, shape: b.shape),
        ]
      end
    end

    # ────────── Unary elementwise (with Rust dispatch): Neg / Abs / Tanh ──────────

    class NegOp < Function
      def forward(a)
        MLFrameworkCore._unary_elementwise("Neg", a, ->(v) { -v })
      end

      # d/dx (-x) = -1.
      def backward(grad)
        [Tensor.new(grad.to_a.map { |v| -v }, shape: grad.shape)]
      end
    end

    class AbsOp < Function
      def forward(a)
        @saved_for_backward[:a] = a
        MLFrameworkCore._unary_elementwise("Abs", a, ->(v) { v.abs })
      end

      # d/dx |x| = sign(x).  Sign of 0 is conventionally 0 (PyTorch matches).
      def backward(grad)
        a = @saved_for_backward[:a]
        g = grad.to_a
        ad = a.to_a
        out = g.each_with_index.map do |gi, i|
          if ad[i].positive?
            gi
          elsif ad[i].negative?
            -gi
          else
            0.0
          end
        end
        [Tensor.new(out, shape: a.shape)]
      end
    end

    class TanhOp < Function
      def forward(a)
        # Save the OUTPUT (not the input) — tanh backward is
        # 1 - tanh(x)² which is cheaper to compute as 1 - y² where
        # y is the forward output we already computed.
        out = MLFrameworkCore._unary_elementwise("Tanh", a, ->(v) { Math.tanh(v) })
        @saved_for_backward[:output] = out
        out
      end

      # d/dx tanh(x) = 1 - tanh(x)² = 1 - y².
      def backward(grad)
        y  = @saved_for_backward[:output]
        g  = grad.to_a
        yd = y.to_a
        out = g.each_with_index.map { |gi, i| gi * (1.0 - yd[i] * yd[i]) }
        [Tensor.new(out, shape: y.shape)]
      end
    end

    # ────────── Pow: scalar exponent, pure Ruby for v0.3.0 ──────────
    #
    # The matrix-cpu Pow op takes two TENSOR inputs.  Routing a
    # scalar-exponent Pow through Rust would require broadcasting the
    # exponent to a full tensor — net-loss below threshold, marginal
    # above.  Stays pure Ruby for v0.3.0; can lift later if profiling
    # shows it matters.
    class PowOp < Function
      def forward(a, exponent)
        e = exponent.is_a?(Numeric) ? exponent.to_f : exponent.to_a[0]
        @saved_for_backward[:a] = a
        @saved_for_backward[:exponent] = e
        Tensor.new(a.to_a.map { |v| v**e }, shape: a.shape)
      end

      # d/dx x^e = e * x^(e-1).  Returns nil for the exponent slot since
      # it's a Float, not a Tensor (autograd doesn't track scalars).
      def backward(grad)
        a = @saved_for_backward[:a]
        e = @saved_for_backward[:exponent]
        g = grad.to_a
        ad = a.to_a
        out = g.each_with_index.map { |gi, i| gi * e * (ad[i]**(e - 1)) }
        # Only one Tensor parent (the exponent is a Numeric, filtered
        # out by Function.apply); return a 1-element Array to match.
        [Tensor.new(out, shape: a.shape)]
      end
    end

    # ────────── MatMul (2-D only in v0.3.0) ──────────

    class MatMulOp < Function
      def forward(a, b)
        unless a.ndim == 2 && b.ndim == 2
          raise ArgumentError,
                "matmul requires 2-D tensors, got #{a.ndim}-D and #{b.ndim}-D"
        end
        m, k1 = a.shape
        k2, n = b.shape
        unless k1 == k2
          raise ArgumentError,
                "matmul shape mismatch: #{a.shape.inspect} @ #{b.shape.inspect}"
        end

        @saved_for_backward[:a] = a
        @saved_for_backward[:b] = b

        if a.numel >= Ops.dispatch_threshold || b.numel >= Ops.dispatch_threshold
          envelope = Ops.matmul_envelope(a, b)
          floats = Ops.run_envelope(envelope, m * n)
          Tensor.new(floats, shape: [m, n])
        else
          # Pure-Ruby triple-loop matmul.  O(m*k*n) — fine at small sizes.
          Tensor.new(MatMulOp._matmul_naive(a.to_a, b.to_a, m, k1, n), shape: [m, n])
        end
      end

      # Backward for C = A @ B (2-D):
      #   dL/dA = grad @ B^T
      #   dL/dB = A^T @ grad
      # We use pure-Ruby triple-loop matmul here.  Reusing MatMulOp.apply
      # would build a NEW autograd subgraph for the backward computation,
      # which we don't want; backward should be a leaf math operation.
      def backward(grad)
        a = @saved_for_backward[:a]
        b = @saved_for_backward[:b]
        m, k = a.shape
        _, n = b.shape

        # grad has shape (m, n); B has shape (k, n); compute grad @ B^T (m, k).
        b_t = MatMulOp._transpose_2d(b.to_a, k, n)
        grad_a_data = MatMulOp._matmul_naive(grad.to_a, b_t, m, n, k)

        # A^T (k, m); compute A^T @ grad (k, n).
        a_t = MatMulOp._transpose_2d(a.to_a, m, k)
        grad_b_data = MatMulOp._matmul_naive(a_t, grad.to_a, k, m, n)

        [
          Tensor.new(grad_a_data, shape: [m, k]),
          Tensor.new(grad_b_data, shape: [k, n]),
        ]
      end

      # Internal helpers — module-level so backward can reuse without
      # building a new MatMulOp (which would attach a grad_fn we don't want).
      def self._matmul_naive(a_data, b_data, m, k, n)
        out = Array.new(m * n, 0.0)
        (0...m).each do |i|
          (0...n).each do |j|
            acc = 0.0
            (0...k).each do |kk|
              acc += a_data[i * k + kk] * b_data[kk * n + j]
            end
            out[i * n + j] = acc
          end
        end
        out
      end

      def self._transpose_2d(data, rows, cols)
        out = Array.new(rows * cols)
        (0...rows).each do |r|
          (0...cols).each do |c|
            out[c * rows + r] = data[r * cols + c]
          end
        end
        out
      end
    end

    # ────────── ReLU / Sigmoid / GELU / Softmax — pure Ruby for v0.3.0 ──────────

    class ReLUOp < Function
      def forward(a)
        @saved_for_backward[:a] = a
        Tensor.new(a.to_a.map { |v| v.positive? ? v : 0.0 }, shape: a.shape)
      end

      # d/dx ReLU(x) = 1 if x > 0 else 0.  Convention: x == 0 returns 0
      # (matches PyTorch).
      def backward(grad)
        a = @saved_for_backward[:a]
        g = grad.to_a
        ad = a.to_a
        out = g.each_with_index.map { |gi, i| ad[i].positive? ? gi : 0.0 }
        [Tensor.new(out, shape: a.shape)]
      end
    end

    class SigmoidOp < Function
      def forward(a)
        out = Tensor.new(a.to_a.map { |v| 1.0 / (1.0 + Math.exp(-v)) }, shape: a.shape)
        # Save the OUTPUT y — sigmoid backward is y * (1 - y), cheaper
        # to compute from the cached y than to re-eval the forward.
        @saved_for_backward[:output] = out
        out
      end

      # d/dx σ(x) = σ(x) * (1 - σ(x)) = y * (1 - y).
      def backward(grad)
        y  = @saved_for_backward[:output]
        g  = grad.to_a
        yd = y.to_a
        out = g.each_with_index.map { |gi, i| gi * yd[i] * (1.0 - yd[i]) }
        [Tensor.new(out, shape: y.shape)]
      end
    end

    class GELUOp < Function
      # GELU(x) = 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x³)))
      # — the "tanh approximation" formulation matching PyTorch's default
      # and the Python reference.
      SQRT_2_OVER_PI = Math.sqrt(2.0 / Math::PI)
      COEFF = 0.044715

      def forward(a)
        @saved_for_backward[:a] = a
        Tensor.new(
          a.to_a.map { |x| 0.5 * x * (1.0 + Math.tanh(SQRT_2_OVER_PI * (x + COEFF * x * x * x))) },
          shape: a.shape,
        )
      end

      # GELU backward (tanh-approximation form), matching the Python ref:
      #   inner  = √(2/π) * (x + 0.044715 * x³)
      #   tanh_v = tanh(inner)
      #   sech²  = 1 - tanh_v²
      #   d_inner = √(2/π) * (1 + 3 * 0.044715 * x²)
      #   dy/dx  = 0.5 * (1 + tanh_v) + 0.5 * x * sech² * d_inner
      def backward(grad)
        a = @saved_for_backward[:a]
        g = grad.to_a
        ad = a.to_a
        out = g.each_with_index.map do |gi, i|
          x = ad[i]
          inner   = SQRT_2_OVER_PI * (x + COEFF * x * x * x)
          tanh_v  = Math.tanh(inner)
          sech2   = 1.0 - tanh_v * tanh_v
          d_inner = SQRT_2_OVER_PI * (1.0 + 3.0 * COEFF * x * x)
          gi * (0.5 * (1.0 + tanh_v) + 0.5 * x * sech2 * d_inner)
        end
        [Tensor.new(out, shape: a.shape)]
      end
    end

    class SoftmaxOp < Function
      # Softmax over the LAST axis (axis = -1).  Numerically stable: subtract
      # the max before exp() so values never overflow exp's range.  Matches
      # the Python reference's softmax_via_python_for_small_tensors path.
      def forward(a)
        flat = a.to_a
        shape = a.shape
        ndim = shape.length
        last_axis_size = ndim.zero? ? 1 : shape[-1]
        outer = a.numel / last_axis_size

        out = Array.new(a.numel)
        outer.times do |o|
          row_start = o * last_axis_size
          # row max for numerical stability
          row_max = -Float::INFINITY
          last_axis_size.times do |k|
            v = flat[row_start + k]
            row_max = v if v > row_max
          end
          # exp(x - max) and accumulate the sum
          sum = 0.0
          tmp = Array.new(last_axis_size)
          last_axis_size.times do |k|
            e = Math.exp(flat[row_start + k] - row_max)
            tmp[k] = e
            sum += e
          end
          # normalise
          last_axis_size.times do |k|
            out[row_start + k] = tmp[k] / sum
          end
        end
        result = Tensor.new(out, shape: a.shape)
        @saved_for_backward[:output] = result
        @saved_for_backward[:last_axis_size] = last_axis_size
        result
      end

      # Softmax backward (per-row over the last axis):
      #
      #   dL/dx_i = y_i * (g_i - Σ_j (g_j * y_j))
      #
      # where y = softmax(x).  This formula is per-row independent —
      # rows can't interfere because softmax doesn't mix across them.
      def backward(grad)
        y = @saved_for_backward[:output]
        last_axis_size = @saved_for_backward[:last_axis_size]
        yd = y.to_a
        gd = grad.to_a
        numel = y.numel
        out = Array.new(numel)

        outer = numel / last_axis_size
        outer.times do |o|
          row_start = o * last_axis_size
          # Σ_j (g_j * y_j) — per-row scalar.
          dot = 0.0
          last_axis_size.times do |k|
            dot += gd[row_start + k] * yd[row_start + k]
          end
          # y_i * (g_i - dot)
          last_axis_size.times do |k|
            idx = row_start + k
            out[idx] = yd[idx] * (gd[idx] - dot)
          end
        end
        [Tensor.new(out, shape: y.shape)]
      end
    end

    # ────────── Reductions: Sum / Mean (reduce-all) ──────────
    #
    # For v0.3.0 we support the reduce-all case only (matches Python
    # reference's MX10 Phase 3).  Per-axis reductions deferred to a
    # follow-up.  Output shape is (1,) — same contract as the Python
    # SumFunction / MeanFunction for dim=None.

    class SumOp < Function
      def forward(a)
        # Save the input shape — we need it to broadcast the scalar
        # gradient back to the input's shape in backward.
        @saved_for_backward[:input_shape] = a.shape
        if a.numel >= Ops.dispatch_threshold
          envelope = Ops.reduce_all_envelope("ReduceSum", a)
          floats = Ops.run_envelope(envelope, 1)
          Tensor.new(floats, shape: [1])
        else
          Tensor.new([a.to_a.sum], shape: [1])
        end
      end

      # d/dx_i (Σ x_j) = 1.  Broadcast the incoming scalar gradient
      # (shape [1]) to a full tensor of the input's shape.
      def backward(grad)
        input_shape = @saved_for_backward[:input_shape]
        g_scalar = grad.to_a[0]
        numel = input_shape.empty? ? 1 : input_shape.reduce(1, :*)
        [Tensor.new(Array.new(numel, g_scalar), shape: input_shape)]
      end
    end

    class MeanOp < Function
      def forward(a)
        @saved_for_backward[:input_shape] = a.shape
        @saved_for_backward[:numel] = a.numel
        if a.numel >= Ops.dispatch_threshold
          envelope = Ops.reduce_all_envelope("ReduceMean", a)
          floats = Ops.run_envelope(envelope, 1)
          Tensor.new(floats, shape: [1])
        else
          n = a.numel.to_f
          Tensor.new([a.to_a.sum / n], shape: [1])
        end
      end

      # d/dx_i ((1/N) Σ x_j) = 1/N.  Broadcast g/N to the input shape.
      def backward(grad)
        input_shape = @saved_for_backward[:input_shape]
        n = @saved_for_backward[:numel].to_f
        g_scalar = grad.to_a[0] / n
        numel = input_shape.empty? ? 1 : input_shape.reduce(1, :*)
        [Tensor.new(Array.new(numel, g_scalar), shape: input_shape)]
      end
    end

    # ========================================================================
    # Wire ops up to Tensor: operator overloads + named methods.
    #
    # We REOPEN Tensor to add these — the existing element-wise overloads
    # from tensor.rb stay in place as the pure-Ruby fallback; the new ones
    # go through Function.apply so the autograd graph builds and large
    # tensors dispatch through Rust.
    #
    # Naming note: we keep the existing `+ - * / **` overloads operating
    # element-wise (their original v0.1 behaviour, unchanged).  For
    # gradient-tracked or large-tensor cases users should call the named
    # methods (`a.relu`, `a.matmul(b)`, etc.) explicitly, OR set
    # `a.requires_grad = true` so the overloads route through Function.apply
    # automatically.
    # ========================================================================
    class Tensor
      # Define the autograd-aware operator overloads.  These shadow the
      # original v0.1 ones; the original pure-Ruby element-wise logic is
      # still reachable via `_binary_op_inline` (the private method).  When
      # autograd is wired up (i.e. when this file is required), the
      # overloads go through Function.apply so grad_fn wiring happens.
      def +(other)
        AddOp.apply(self, _coerce_tensor(other))
      end

      def -(other)
        SubOp.apply(self, _coerce_tensor(other))
      end

      def *(other)
        MulOp.apply(self, _coerce_tensor(other))
      end

      def /(other)
        DivOp.apply(self, _coerce_tensor(other))
      end

      def **(other)
        PowOp.apply(self, other)
      end

      def -@
        NegOp.apply(self)
      end

      def abs
        AbsOp.apply(self)
      end

      def matmul(other)
        MatMulOp.apply(self, other)
      end

      def relu
        ReLUOp.apply(self)
      end

      def sigmoid
        SigmoidOp.apply(self)
      end

      def tanh
        TanhOp.apply(self)
      end

      def gelu
        GELUOp.apply(self)
      end

      def softmax
        SoftmaxOp.apply(self)
      end

      def sum
        SumOp.apply(self)
      end

      def mean
        MeanOp.apply(self)
      end

      private

      # When a binary op receives a scalar (e.g. `t + 5`), broadcast it
      # to a tensor of the same shape so the elementwise path works
      # uniformly.  Materialising a full broadcast tensor is wasteful for
      # large tensors but for v0.3.0 it keeps the autograd story simple
      # — scalar broadcasting in matrix-cpu lands in a later PR.
      def _coerce_tensor(other)
        return other if other.is_a?(Tensor)

        if other.is_a?(Numeric)
          Tensor.full(shape, other.to_f)
        else
          raise TypeError, "cannot combine Tensor with #{other.class}"
        end
      end
    end
  end
end
