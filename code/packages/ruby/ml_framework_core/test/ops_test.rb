# frozen_string_literal: true

# ops_test.rb — coverage for the 15 differentiable ops added in PR #6.
# ======================================================================
#
# Test sections:
#   - HexHelpersTest — pack_f32_hex / unpack_f32_hex round-trip
#   - ForwardSmallTest — all 15 ops, small tensors (pure-Ruby path)
#   - AutogradWiringTest — every op produces a tensor with grad_fn
#     when input has requires_grad
#   - OperatorOverloadsTest — `+ - * / ** -` now route through Function.apply
#   - DispatchPathBranchingTest — verifies the threshold gate (we don't
#     actually invoke Rust here, just confirm the small-tensor path)
#
# We DON'T exercise the Rust dispatch path in this test file because it
# requires the matrix_rust_ruby gem's native ext to be built — that's
# integration territory.  The Python reference has a separate
# `test_rust_backend_parity.py` for that comparison.
#
# Gradient correctness (backward()) is PR #7's responsibility; this file
# only checks that `forward` produces correct output values and that
# `grad_fn` is wired up so the graph EXISTS for PR #7 to traverse.

require "minitest/autorun"
require "coding_adventures/ml_framework_core"

T   = CodingAdventures::MLFrameworkCore::Tensor   unless defined?(T)
Ops = CodingAdventures::MLFrameworkCore::Ops      unless defined?(Ops)

# Op constant shortcuts.
M    = CodingAdventures::MLFrameworkCore
AddO = M::AddOp
SubO = M::SubOp
MulO = M::MulOp
DivO = M::DivOp
NegO = M::NegOp
AbsO = M::AbsOp
PowO = M::PowOp
MMo  = M::MatMulOp
ReLU = M::ReLUOp
Sig  = M::SigmoidOp
Tnh  = M::TanhOp
Gel  = M::GELUOp
Sfx  = M::SoftmaxOp
SmO  = M::SumOp
MnO  = M::MeanOp

# Float comparison helper — f32 dispatch chops precision somewhere
# around 1e-7, so use a generous epsilon for transcendentals.
def assert_arrays_close(expected, actual, epsilon: 1e-6, msg: nil)
  assert_equal expected.length, actual.length, "length mismatch: #{msg}"
  expected.each_with_index do |e, i|
    a = actual[i]
    delta = (e - a).abs
    assert delta < epsilon, "#{msg} [#{i}]: expected #{e}, got #{a} (delta #{delta} > #{epsilon})"
  end
end

class HexHelpersTest < Minitest::Test
  def test_pack_then_unpack_round_trip
    arr = [1.5, 2.5, -3.25, 0.0, 1.0]
    hex = Ops.pack_f32_hex(arr)
    back = Ops.unpack_f32_hex(hex, arr.length)
    assert_arrays_close arr, back, msg: "round-trip"
  end

  def test_pack_emits_4_hex_chars_per_byte_per_cell
    # Single f32 = 4 bytes = 8 hex chars.
    hex = Ops.pack_f32_hex([1.0])
    assert_equal 8, hex.length, "expected 8 hex chars (4 bytes × 2), got #{hex.inspect}"
  end

  def test_pack_known_value
    # 1.0 in little-endian f32 is 00 00 80 3f.
    assert_equal "0000803f", Ops.pack_f32_hex([1.0])
  end

  def test_unpack_known_hex
    arr = Ops.unpack_f32_hex("0000803f", 1)
    assert_equal [1.0], arr
  end
end

class ForwardSmallTest < Minitest::Test
  def test_add
    a = T.new([1.0, 2.0, 3.0])
    b = T.new([10.0, 20.0, 30.0])
    assert_equal [11.0, 22.0, 33.0], AddO.apply(a, b).to_a
  end

  def test_sub
    a = T.new([10.0, 20.0])
    b = T.new([1.0, 2.0])
    assert_equal [9.0, 18.0], SubO.apply(a, b).to_a
  end

  def test_mul
    a = T.new([2.0, 3.0])
    b = T.new([4.0, 5.0])
    assert_equal [8.0, 15.0], MulO.apply(a, b).to_a
  end

  def test_div
    a = T.new([10.0, 20.0])
    b = T.new([2.0, 4.0])
    assert_equal [5.0, 5.0], DivO.apply(a, b).to_a
  end

  def test_neg
    a = T.new([1.0, -2.0, 3.0])
    assert_equal [-1.0, 2.0, -3.0], NegO.apply(a).to_a
  end

  def test_abs
    a = T.new([-1.5, 2.5, -3.5])
    assert_equal [1.5, 2.5, 3.5], AbsO.apply(a).to_a
  end

  def test_pow_scalar_exponent
    a = T.new([2.0, 3.0, 4.0])
    assert_equal [4.0, 9.0, 16.0], PowO.apply(a, 2).to_a
  end

  def test_matmul_small_2x2
    a = T.new([[1, 2], [3, 4]])
    b = T.new([[5, 6], [7, 8]])
    # [1*5+2*7, 1*6+2*8] = [19, 22]
    # [3*5+4*7, 3*6+4*8] = [43, 50]
    out = MMo.apply(a, b)
    assert_equal [2, 2], out.shape
    assert_equal [19.0, 22.0, 43.0, 50.0], out.to_a
  end

  def test_matmul_rejects_non_2d
    assert_raises(ArgumentError) { MMo.apply(T.new([1, 2, 3]), T.new([[1, 2], [3, 4]])) }
  end

  def test_matmul_rejects_inner_dim_mismatch
    assert_raises(ArgumentError) { MMo.apply(T.zeros(2, 3), T.zeros(4, 5)) }
  end

  def test_relu
    a = T.new([-1.0, 0.0, 1.0, -2.5, 3.5])
    assert_equal [0.0, 0.0, 1.0, 0.0, 3.5], ReLU.apply(a).to_a
  end

  def test_sigmoid_at_zero_is_half
    out = Sig.apply(T.new([0.0])).to_a
    assert_in_delta 0.5, out[0], 1e-12
  end

  def test_sigmoid_monotonic
    out = Sig.apply(T.new([-2.0, -1.0, 0.0, 1.0, 2.0])).to_a
    out.each_cons(2) { |a, b| assert b >= a, "sigmoid must be monotonic" }
  end

  def test_tanh_at_zero_is_zero
    assert_in_delta 0.0, Tnh.apply(T.new([0.0])).to_a[0], 1e-12
  end

  def test_tanh_approaches_one_for_large_positive
    assert_in_delta 1.0, Tnh.apply(T.new([10.0])).to_a[0], 1e-6
  end

  def test_gelu_at_zero_is_zero
    assert_in_delta 0.0, Gel.apply(T.new([0.0])).to_a[0], 1e-12
  end

  def test_gelu_at_one
    # GELU(1) ≈ 0.8413 (tanh-approx form)
    out = Gel.apply(T.new([1.0])).to_a[0]
    assert_in_delta 0.8413, out, 1e-3
  end

  def test_softmax_sums_to_one
    out = Sfx.apply(T.new([1.0, 2.0, 3.0, 4.0])).to_a
    assert_in_delta 1.0, out.sum, 1e-6
  end

  def test_softmax_largest_input_largest_output
    out = Sfx.apply(T.new([1.0, 2.0, 3.0, 4.0])).to_a
    assert_equal out.max, out[3]
    assert_equal out.min, out[0]
  end

  def test_softmax_numerical_stability_large_inputs
    # Without the max-subtraction trick, exp(1000) overflows to Infinity.
    # Our impl subtracts the max first, so this must NOT raise or NaN.
    out = Sfx.apply(T.new([1000.0, 1001.0, 1002.0])).to_a
    out.each { |v| assert v.finite?, "softmax of large inputs gave non-finite: #{v}" }
    assert_in_delta 1.0, out.sum, 1e-6
  end

  def test_sum_returns_scalar_shape_1
    out = SmO.apply(T.new([1.0, 2.0, 3.0, 4.0]))
    assert_equal [1], out.shape
    assert_equal [10.0], out.to_a
  end

  def test_mean_returns_scalar_shape_1
    out = MnO.apply(T.new([1.0, 2.0, 3.0, 4.0]))
    assert_equal [1], out.shape
    assert_equal [2.5], out.to_a
  end
end

class AutogradWiringTest < Minitest::Test
  # For each op: input with requires_grad → output has grad_fn of the
  # right class.  Just verifies the wiring; gradient values are PR #7.

  def test_add_wires_grad_fn
    x = T.new([1.0, 2.0]); x.requires_grad = true
    y = T.new([3.0, 4.0]); y.requires_grad = true
    z = AddO.apply(x, y)
    assert z.requires_grad
    assert_kind_of AddO, z.grad_fn
    assert_equal [x, y], z.grad_fn.parents
  end

  def test_unary_ops_wire_grad_fn
    x = T.new([1.0, 2.0]); x.requires_grad = true
    [NegO, AbsO, ReLU, Sig, Tnh, Gel, Sfx].each do |op|
      y = op.apply(x)
      assert y.requires_grad, "#{op}.apply output should require_grad"
      assert_kind_of op, y.grad_fn, "#{op}.apply output should have #{op} grad_fn"
    end
  end

  def test_matmul_wires_grad_fn
    a = T.zeros(2, 3); a.requires_grad = true
    b = T.zeros(3, 2)
    out = MMo.apply(a, b)
    assert out.requires_grad
    assert_kind_of MMo, out.grad_fn
  end

  def test_reductions_wire_grad_fn
    x = T.new([1.0, 2.0, 3.0]); x.requires_grad = true
    assert_kind_of SmO, SmO.apply(x).grad_fn
    assert_kind_of MnO, MnO.apply(x).grad_fn
  end

  def test_no_grad_fn_when_no_input_requires_grad
    z = AddO.apply(T.new([1.0]), T.new([2.0]))
    refute z.requires_grad
    assert_nil z.grad_fn
  end
end

class OperatorOverloadsTest < Minitest::Test
  # The original v0.1 overloads used inline element-wise math; the new
  # ones in ops.rb shadow them and dispatch through Function.apply.
  # Numeric values are unchanged (Tensor#== is value-based) but they now
  # build an autograd graph.

  def test_plus_routes_through_add_op
    x = T.new([1.0, 2.0]); x.requires_grad = true
    y = T.new([3.0, 4.0])
    z = x + y
    assert_equal [4.0, 6.0], z.to_a
    assert_kind_of AddO, z.grad_fn
  end

  def test_unary_minus_routes_through_neg_op
    x = T.new([1.0, -2.0]); x.requires_grad = true
    y = -x
    assert_equal [-1.0, 2.0], y.to_a
    assert_kind_of NegO, y.grad_fn
  end

  def test_scalar_broadcast_via_coercion
    # `t + 5` should broadcast 5 into a same-shape tensor before Add.
    x = T.new([1.0, 2.0, 3.0])
    y = x + 5
    assert_equal [6.0, 7.0, 8.0], y.to_a
  end

  def test_pow_with_scalar
    x = T.new([2.0, 3.0])
    y = x**3
    assert_equal [8.0, 27.0], y.to_a
  end

  def test_unsupported_operand_raises
    assert_raises(TypeError) { T.new([1.0]) + "not a number" }
  end
end

class TensorNamedOpMethodsTest < Minitest::Test
  def test_relu_method_dispatches_to_relu_op
    x = T.new([-1.0, 0.0, 1.0]); x.requires_grad = true
    y = x.relu
    assert_equal [0.0, 0.0, 1.0], y.to_a
    assert_kind_of ReLU, y.grad_fn
  end

  def test_sigmoid_method
    out = T.new([0.0]).sigmoid.to_a[0]
    assert_in_delta 0.5, out, 1e-12
  end

  def test_tanh_method
    assert_in_delta 0.0, T.new([0.0]).tanh.to_a[0], 1e-12
  end

  def test_gelu_method
    assert_in_delta 0.0, T.new([0.0]).gelu.to_a[0], 1e-12
  end

  def test_softmax_method
    out = T.new([1.0, 1.0, 1.0]).softmax.to_a
    assert_arrays_close [1.0 / 3, 1.0 / 3, 1.0 / 3], out, epsilon: 1e-6
  end

  def test_sum_method
    assert_equal [6.0], T.new([1.0, 2.0, 3.0]).sum.to_a
  end

  def test_mean_method
    assert_equal [2.0], T.new([1.0, 2.0, 3.0]).mean.to_a
  end

  def test_matmul_method
    a = T.new([[1, 2], [3, 4]])
    b = T.new([[1, 0], [0, 1]])
    assert_equal a.to_a, a.matmul(b).to_a
  end

  def test_abs_method
    assert_equal [1.0, 2.0, 3.0], T.new([-1.0, 2.0, -3.0]).abs.to_a
  end
end

class DispatchPathBranchingTest < Minitest::Test
  def test_dispatch_threshold_constant
    # Smoke check that the threshold module method returns a sensible value.
    assert_kind_of Integer, Ops.dispatch_threshold
    assert Ops.dispatch_threshold >= 1
    assert_equal 10_000, Ops.dispatch_threshold
  end

  def test_small_tensor_uses_ruby_fallback
    # If small-tensor path tried to dispatch through Rust, this would
    # require the matrix_rust_ruby native ext.  Since we KNOW we're below
    # threshold, no MatrixRustRuby require should fire.  Easiest proof:
    # the operation completes without ::MatrixRustRuby being defined.
    refute defined?(::MatrixRustRuby), "MatrixRustRuby should not be loaded yet"
    # Run a small Add — must stay pure Ruby.
    result = AddO.apply(T.new([1.0, 2.0]), T.new([3.0, 4.0]))
    assert_equal [4.0, 6.0], result.to_a
    # Still not loaded.
    refute defined?(::MatrixRustRuby), "small Add must not have loaded MatrixRustRuby"
  end
end
