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
    end

    class SubOp < Function
      def forward(a, b)
        MLFrameworkCore._binary_elementwise("Sub", a, b, ->(x, y) { x - y })
      end
    end

    class MulOp < Function
      def forward(a, b)
        MLFrameworkCore._binary_elementwise("Mul", a, b, ->(x, y) { x * y })
      end
    end

    class DivOp < Function
      def forward(a, b)
        MLFrameworkCore._binary_elementwise("Div", a, b, ->(x, y) { x / y })
      end
    end

    # ────────── Unary elementwise (with Rust dispatch): Neg / Abs / Tanh ──────────

    class NegOp < Function
      def forward(a)
        MLFrameworkCore._unary_elementwise("Neg", a, ->(v) { -v })
      end
    end

    class AbsOp < Function
      def forward(a)
        MLFrameworkCore._unary_elementwise("Abs", a, ->(v) { v.abs })
      end
    end

    class TanhOp < Function
      def forward(a)
        MLFrameworkCore._unary_elementwise("Tanh", a, ->(v) { Math.tanh(v) })
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
        Tensor.new(a.to_a.map { |v| v**e }, shape: a.shape)
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

        if a.numel >= Ops.dispatch_threshold || b.numel >= Ops.dispatch_threshold
          envelope = Ops.matmul_envelope(a, b)
          floats = Ops.run_envelope(envelope, m * n)
          Tensor.new(floats, shape: [m, n])
        else
          # Pure-Ruby triple-loop matmul.  O(m*k*n) — fine at small sizes.
          a_data = a.to_a
          b_data = b.to_a
          out = Array.new(m * n, 0.0)
          (0...m).each do |i|
            (0...n).each do |j|
              acc = 0.0
              (0...k1).each do |kk|
                acc += a_data[i * k1 + kk] * b_data[kk * n + j]
              end
              out[i * n + j] = acc
            end
          end
          Tensor.new(out, shape: [m, n])
        end
      end
    end

    # ────────── ReLU / Sigmoid / GELU / Softmax — pure Ruby for v0.3.0 ──────────

    class ReLUOp < Function
      def forward(a)
        Tensor.new(a.to_a.map { |v| v.positive? ? v : 0.0 }, shape: a.shape)
      end
    end

    class SigmoidOp < Function
      def forward(a)
        Tensor.new(a.to_a.map { |v| 1.0 / (1.0 + Math.exp(-v)) }, shape: a.shape)
      end
    end

    class GELUOp < Function
      # GELU(x) = 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x³)))
      # — the "tanh approximation" formulation matching PyTorch's default
      # and the Python reference.
      def forward(a)
        c = Math.sqrt(2.0 / Math::PI)
        Tensor.new(
          a.to_a.map { |x| 0.5 * x * (1.0 + Math.tanh(c * (x + 0.044715 * x * x * x))) },
          shape: a.shape,
        )
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
        Tensor.new(out, shape: a.shape)
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
        if a.numel >= Ops.dispatch_threshold
          envelope = Ops.reduce_all_envelope("ReduceSum", a)
          floats = Ops.run_envelope(envelope, 1)
          Tensor.new(floats, shape: [1])
        else
          Tensor.new([a.to_a.sum], shape: [1])
        end
      end
    end

    class MeanOp < Function
      def forward(a)
        if a.numel >= Ops.dispatch_threshold
          envelope = Ops.reduce_all_envelope("ReduceMean", a)
          floats = Ops.run_envelope(envelope, 1)
          Tensor.new(floats, shape: [1])
        else
          n = a.numel.to_f
          Tensor.new([a.to_a.sum / n], shape: [1])
        end
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
