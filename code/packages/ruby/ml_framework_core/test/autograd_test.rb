# frozen_string_literal: true

# autograd_test.rb — exercises Function.apply + Tensor#backward
# =============================================================
#
# These tests use only the `Identity` Function subclass (forward = clone,
# backward = pass-through) so the test suite is decoupled from any
# op-specific math.  PR #6 adds the real ops and their parity/gradient
# tests; this file just proves the *machinery* works.
#
# Categories:
#   - apply() wiring: requires_grad propagation, grad_fn attachment
#   - Tensor#backward sanity: must require requires_grad, must accept seed
#   - End-to-end: backward populates leaf .grad
#   - Accumulation: repeated backward sums grads
#   - Chain: multi-step graphs propagate correctly
#   - Shared parents: `x → Identity → y, x → Identity → z` accumulates

require "minitest/autorun"
require "coding_adventures/ml_framework_core"

T  = CodingAdventures::MLFrameworkCore::Tensor unless defined?(T)
Fn = CodingAdventures::MLFrameworkCore::Function
Id = CodingAdventures::MLFrameworkCore::Identity

class AutogradApplyTest < Minitest::Test
  def test_requires_grad_propagates_through_identity
    x = T.new([1.0, 2.0, 3.0])
    x.requires_grad = true
    y = Id.apply(x)

    assert y.requires_grad, "output should inherit requires_grad from input"
    assert_kind_of Id, y.grad_fn
    assert_equal [x], y.grad_fn.parents
  end

  def test_no_grad_fn_when_no_input_requires_grad
    x = T.new([1.0, 2.0, 3.0])    # requires_grad defaults to false
    y = Id.apply(x)

    refute y.requires_grad
    assert_nil y.grad_fn
  end

  def test_apply_returns_a_new_tensor
    x = T.new([1.0, 2.0])
    x.requires_grad = true
    y = Id.apply(x)
    refute y.equal?(x), "Identity#forward returns a new tensor, not the same object"
    assert_equal x, y, "...but the data should compare equal"
  end

  def test_function_subclass_must_implement_forward
    bad_class = Class.new(Fn)
    assert_raises(NotImplementedError) { bad_class.apply(T.new([1.0])) }
  end

  def test_function_subclass_must_implement_backward
    # Anonymous subclass — has to fully qualify Tensor because it's NOT
    # nested under the MLFrameworkCore module (where bare `Tensor` would
    # resolve via constant lookup).
    no_back = Class.new(Fn) do
      def forward(x)
        CodingAdventures::MLFrameworkCore::Tensor.new(x.to_a, shape: x.shape)
      end
    end
    x = T.new([1.0]); x.requires_grad = true
    y = no_back.apply(x)
    assert_raises(NotImplementedError) { y.backward }
  end
end

class TensorBackwardSanityTest < Minitest::Test
  def test_backward_on_non_grad_tensor_raises
    x = T.new([1.0, 2.0, 3.0])
    refute x.requires_grad
    assert_raises(RuntimeError) { x.backward }
  end

  def test_backward_with_mismatched_grad_shape_raises
    x = T.new([1.0, 2.0]); x.requires_grad = true
    y = Id.apply(x)
    assert_raises(ArgumentError) { y.backward(T.ones(3)) }   # wrong shape
  end

  def test_backward_returns_nil
    x = T.new([1.0, 2.0]); x.requires_grad = true
    y = Id.apply(x)
    assert_nil y.backward
  end
end

class AutogradEndToEndTest < Minitest::Test
  def test_identity_backward_writes_ones_to_leaf_grad
    x = T.new([1.0, 2.0, 3.0]); x.requires_grad = true
    y = Id.apply(x)
    y.backward
    # d(identity(x))/dx == 1 everywhere; with seed grad = ones, x.grad = ones.
    assert_equal T.ones(3), x.grad
  end

  def test_identity_backward_with_explicit_seed_grad
    x = T.new([1.0, 2.0]); x.requires_grad = true
    y = Id.apply(x)
    y.backward(T.full([2], 5.0))
    # Identity's local derivative is 1; seed grad = [5, 5] passes through.
    assert_equal T.full([2], 5.0), x.grad
  end

  def test_backward_twice_accumulates_into_leaf_grad
    x = T.new([1.0, 2.0]); x.requires_grad = true
    y = Id.apply(x)
    y.backward
    y.backward
    # PyTorch convention: repeated backward sums into .grad (caller is
    # expected to zero_grad between training steps).
    assert_equal T.full([2], 2.0), x.grad
  end

  def test_chain_of_identities_propagates
    x = T.new([1.0, 2.0, 3.0]); x.requires_grad = true
    y = Id.apply(Id.apply(Id.apply(x)))
    y.backward
    # Chain rule with all-identity is still all-ones.
    assert_equal T.ones(3), x.grad
  end

  def test_shared_parent_accumulates_from_both_paths
    # Build:    x ─┬─ Id ─→ a
    #              └─ Id ─→ b
    # Then put a and b into a single op so backprop visits both paths.
    # The easiest way without op-dispatch is to manually invoke backward
    # on a, then on b, and confirm the SAME leaf x.grad accumulates.
    x = T.new([1.0, 2.0]); x.requires_grad = true
    a = Id.apply(x)
    b = Id.apply(x)
    a.backward
    b.backward
    # Two ones-vectors summed into x.grad.
    assert_equal T.full([2], 2.0), x.grad
  end

  def test_leaf_node_without_requires_grad_is_skipped
    # An intermediate non-grad input shouldn't be assigned a grad.
    x = T.new([1.0, 2.0])          # no requires_grad → leaf, no grad
    y = T.new([1.0, 2.0]); y.requires_grad = true
    z1 = Id.apply(x)
    _z2 = z1                       # no chain back to y; just verify x.grad stays nil
    refute z1.requires_grad
    assert_nil x.grad

    # And confirm a grad-tracking branch still works in isolation.
    w = Id.apply(y)
    w.backward
    assert_equal T.ones(2), y.grad
  end
end

class AutogradFunctionIntrospectionTest < Minitest::Test
  def test_function_inspect_shows_class_and_parent_count
    x = T.new([1.0]); x.requires_grad = true
    y = Id.apply(x)
    assert_match(/Identity/, y.grad_fn.inspect)
    assert_match(/parents=1/, y.grad_fn.inspect)
  end

  def test_function_init_has_empty_parents_and_saved
    fn = Fn.new
    assert_equal [], fn.parents
    assert_equal({}, fn.saved_for_backward)
  end
end

class TensorHelperFactoriesTest < Minitest::Test
  def test_ones_like
    x = T.zeros(2, 3)
    o = T.ones_like(x)
    assert_equal x.shape, o.shape
    assert_equal [1.0] * 6, o.to_a
  end

  def test_zeros_like
    x = T.ones(4)
    z = T.zeros_like(x)
    assert_equal x.shape, z.shape
    assert_equal [0.0] * 4, z.to_a
  end
end
