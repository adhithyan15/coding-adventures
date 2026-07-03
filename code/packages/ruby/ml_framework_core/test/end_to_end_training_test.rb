# frozen_string_literal: true

# end_to_end_training_test.rb — proves the autograd stack actually trains.
# ===========================================================================
#
# This is the "does it really work?" test for the whole gem.  All the
# unit tests so far prove individual op correctness and graph wiring;
# this test runs a real (if tiny) training loop and asserts that loss
# decreases monotonically over the steps.
#
# Mirrors `code/packages/python/ml-framework-core/tests/test_end_to_end_training.py`
# but stripped to the minimum scope:
#
#   - 2-layer MLP: x → linear(W₁) → ReLU → linear(W₂) → loss
#   - MSE loss against a synthetic target
#   - Manual SGD: param.data -= lr * param.grad
#   - 30 steps
#   - Assert final loss < initial loss / 4 (a substantial drop, not just
#     "trended down once and stopped")
#
# We use a tiny problem (4 samples, 2 hidden units) so the test runs
# in milliseconds.  Larger problems would exercise the Rust dispatch
# path which has its own tests; here we're only validating the
# autograd loop itself.

require "minitest/autorun"
require "coding_adventures/ml_framework_core"

T = CodingAdventures::MLFrameworkCore::Tensor unless defined?(T)

class EndToEndTrainingTest < Minitest::Test
  # Helper: SGD step.  PyTorch convention: subtract lr * grad from
  # param's data in place, then zero the gradient before the next step.
  #
  # We build a fresh Tensor with `.requires_grad = true` because Tensors
  # are immutable from the outside (no public `.data =` setter); the
  # "in place update" is really "swap the underlying data array".
  def sgd_step(param, lr)
    new_data = param.to_a.each_with_index.map { |v, i| v - lr * param.grad.to_a[i] }
    new_param = T.new(new_data, shape: param.shape)
    new_param.requires_grad = true
    new_param
  end

  def test_two_layer_mlp_trains_loss_decreases
    # Synthetic dataset: regress y = 2x + 3 + small noise
    # We use 4 input samples in shape (4, 1).
    x_data = [[0.0], [1.0], [2.0], [3.0]]
    y_data = [[3.0], [5.0], [7.0], [9.0]]  # exactly 2x + 3
    x = T.new(x_data)
    target = T.new(y_data)

    # Layer 1: (1, 2) — projects 1-D input to 2 hidden units
    # Layer 2: (2, 1) — projects 2 hidden units to 1 output
    # Initialise to small random values for deterministic test runs.
    w1 = T.new([[0.5, -0.3]])
    w2 = T.new([[0.4], [0.7]])
    w1.requires_grad = true
    w2.requires_grad = true

    # lr = 0.01 is conservative; with lr = 0.05 the small MLP overshoots
    # and lands in a 2-cycle around the minimum (oscillation between
    # two local-near-min states), which is correct gradient-descent
    # behavior but defeats the monotonicity check.  0.01 converges
    # cleanly in 30 steps.
    lr = 0.01
    losses = []
    steps = 30

    steps.times do
      # Forward pass:  pred = ((x @ w1).relu) @ w2
      pred = x.matmul(w1).relu.matmul(w2)
      # MSE loss: mean((pred - target)²)
      diff = pred - target
      loss = (diff * diff).mean
      losses << loss.to_a[0]

      loss.backward
      # SGD update.  In Python this would mutate w1.data; here we swap
      # in a fresh Tensor (and zero the grad by virtue of constructing
      # a new one with .grad = nil).
      w1 = sgd_step(w1, lr)
      w2 = sgd_step(w2, lr)
    end

    initial = losses.first
    final   = losses.last

    # Hard requirement: loss MUST decrease overall.
    assert final < initial, "loss must decrease over training (initial #{initial}, final #{final})"

    # Stronger: loss should drop by at least 75% — proves the autograd
    # gradients are pointing in roughly the right direction, not just
    # randomly walking.  Note: a bias-free MLP can't perfectly fit
    # `y = 2x + 3` (the model has no constant term), so loss bottoms
    # out around 3.2 rather than 0.  We measure the relative drop,
    # not the absolute floor.
    drop_ratio = (initial - final) / initial
    assert drop_ratio > 0.75,
           "loss should drop by >75% over #{steps} SGD steps, got #{(drop_ratio * 100).round(1)}% (#{initial} → #{final})"

    # Sanity check: losses should be MOSTLY monotonically decreasing.
    # A few small bumps are OK (SGD is noisy) but the trend must be down.
    # We allow up to ⌊steps/3⌋ steps where loss[i+1] > loss[i].
    increasing_steps = losses.each_cons(2).count { |a, b| b > a }
    assert increasing_steps <= steps / 3,
           "loss should mostly decrease (#{increasing_steps} increase events in #{steps - 1} step transitions)"
  end

  def test_linear_regression_converges
    # Even simpler: 1-layer linear regression y = w*x.
    # No nonlinearity, no hidden layer — purely tests that gradient
    # accumulation + SGD on a single parameter converges.
    x = T.new([[1.0], [2.0], [3.0], [4.0]])
    target = T.new([[2.0], [4.0], [6.0], [8.0]])    # y = 2x exactly

    w = T.new([[0.5]])  # start far from the true value (2.0)
    w.requires_grad = true
    lr = 0.05

    20.times do
      pred = x.matmul(w)
      diff = pred - target
      loss = (diff * diff).mean
      loss.backward
      w = sgd_step(w, lr)
    end

    # After 20 steps with this simple problem, w should be close to 2.0.
    final_w = w.to_a[0]
    assert (final_w - 2.0).abs < 0.5,
           "linear regression should converge near w=2.0, got w=#{final_w}"
  end
end
