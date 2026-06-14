# frozen_string_literal: true

# autograd.rb — reverse-mode automatic differentiation for Tensor
# =================================================================
#
# This file adds two pieces of machinery to ml_framework_core:
#
#   1. The `Function` base class.  Every differentiable op (Add, MatMul,
#      ReLU, ...) is a `Function` subclass that defines `forward` and
#      `backward`.  PR #6 implements the ~15 specific subclasses; here
#      we just provide the base class and one Identity subclass for
#      testing the machinery.
#
#   2. `Tensor#backward(grad = nil)` — kicks off backpropagation from a
#      tensor (typically a scalar loss).  Walks the autograd graph in
#      reverse topological order, calling each Function's `backward` to
#      compute input gradients and accumulating them into leaf tensors'
#      `grad` slot.
#
# # Why it's so small
#
# All the heavy lifting in autograd is per-op math (PR #6/#7).  The
# *framework* itself — apply() plumbing + topo sort + reverse walk — is
# under 100 lines.  This is a property of well-designed reverse-mode AD:
# every paper that ever described autograd shows this exact algorithm.
#
# # Mirrors the Python structure
#
# This file deliberately mirrors
# `code/packages/python/ml-framework-core/src/ml_framework_core/autograd.py`
# in:
#   - Function base class shape (saved_tensors + saved_metadata)
#   - apply()'s steps: probe requires_grad → run forward → wire grad_fn
#   - backward()'s topo-sort + reverse-walk algorithm
#   - gradient accumulation rule (sum across paths into a node)
#
# Differences are purely Ruby idiom:
#   - `id()` → `object_id`
#   - `dict` → `Hash`
#   - `set` → `Set` (or just keying a Hash on object_id)
#   - module function `backward(tensor)` → `Tensor#backward` method

require "set"
require_relative "tensor"

module CodingAdventures
  module MLFrameworkCore
    # ========================================================================
    # Reopen Tensor — autograd needs setters for grad / grad_fn and a
    # `Tensor.ones_like` convenience factory.  Keeping them here (instead
    # of in tensor.rb) keeps the v0.1 Tensor PR strictly storage-only,
    # and concentrates the autograd-related additions in one file for
    # easy review.
    # ========================================================================
    class Tensor
      # Setters for the autograd-only attributes.  Public so Function.apply
      # and the backward() walker can write them; users shouldn't need to.
      attr_writer :grad, :grad_fn

      # `Tensor.ones_like(t)` — a tensor of 1.0s with the same shape as `t`.
      # The PyTorch convention for `backward()` with no explicit gradient.
      def self.ones_like(other)
        ones(*other.shape)
      end

      # `Tensor.zeros_like(t)` — companion to ones_like; used here for the
      # leaf-grad initialization branch in backward().
      def self.zeros_like(other)
        zeros(*other.shape)
      end

      # Tensor#backward(grad = nil) — kick off backprop.
      #
      # Algorithm:
      #   1. If grad is nil, default to ones-like (PyTorch convention; valid
      #      strictly only for scalar tensors but we allow any shape here
      #      for simplicity, matching what PR #6's tests will need).
      #   2. Topological sort upward through grad_fn chain — DFS, post-order.
      #   3. Walk topo list in reverse; for each tensor with a grad_fn,
      #      call grad_fn.backward(tensor.grad), get input grads, accumulate
      #      into each parent's grad slot via the grad_map.
      #   4. For leaf tensors with requires_grad, copy the accumulated
      #      gradient into the tensor's `grad` slot (accumulating if a
      #      previous backward already wrote there — supports repeated
      #      backward() calls without zero_grad in between).
      #
      # Returns nil (mutates in place; matches PyTorch).
      def backward(grad = nil)
        unless requires_grad
          raise RuntimeError, "backward() called on a tensor that doesn't require grad"
        end

        grad ||= Tensor.ones_like(self)
        unless grad.shape == shape
          raise ArgumentError,
                "backward grad shape #{grad.shape.inspect} != tensor shape #{shape.inspect}"
        end

        # Topological order: each tensor appears AFTER its parents in
        # the list, so walking in REVERSE processes children before parents.
        topo_order = []
        visited    = Set.new
        build_topo = lambda do |t|
          # Visiting set keyed by object_id so duplicate sub-graphs (e.g.
          # `x + x`) are handled correctly — visit each Tensor exactly once.
          oid = t.object_id
          next if visited.include?(oid)

          visited.add(oid)
          if t.grad_fn
            t.grad_fn.parents.each { |p| build_topo.call(p) }
          end
          topo_order << t
        end
        build_topo.call(self)

        # grad_map: tensor.object_id → accumulated gradient Tensor.
        # Starts with our seed gradient at the root.
        grad_map = { object_id => grad }

        topo_order.reverse_each do |node|
          node_grad = grad_map[node.object_id]
          next if node_grad.nil?

          if node.grad_fn.nil?
            # Leaf tensor — store/accumulate into its public grad slot.
            next unless node.requires_grad

            if node.grad.nil?
              node.grad = Tensor.new(node_grad.to_a, shape: node_grad.shape)
            else
              # Accumulate: support repeated backward() calls (e.g. when
              # training a model and not calling zero_grad between steps).
              node.grad = node.grad + node_grad
            end
            next
          end

          # Non-leaf — ask the Function for input grads and distribute them
          # into the grad_map for the parents.
          input_grads = node.grad_fn.backward(node_grad)
          input_grads = [input_grads] unless input_grads.is_a?(Array)
          node.grad_fn.parents.each_with_index do |parent, i|
            input_grad = input_grads[i]
            next if input_grad.nil?

            existing = grad_map[parent.object_id]
            grad_map[parent.object_id] =
              if existing.nil?
                input_grad
              else
                # Two paths converged on this parent — sum the gradients.
                # (E.g. in `x + x`, the same `x` reaches the add twice.)
                existing + input_grad
              end
          end
        end

        nil
      end
    end

    # ========================================================================
    # The Function base class — extend this for every differentiable op.
    # ========================================================================
    class Function
      # @return [Array<Tensor>] the input tensors that produced the output.
      #   backward() distributes gradients to these in the same order.
      attr_accessor :parents

      # @return [Hash{Symbol => Object}] subclass-defined values cached in
      #   forward() for use in backward() — typically input shapes, the
      #   activation values needed for derivative computation (e.g.
      #   sigmoid backward needs the output value), etc.
      attr_accessor :saved_for_backward

      def initialize
        @parents = []
        @saved_for_backward = {}
      end

      # Class method — the canonical entry point for invoking an op.
      #
      # Algorithm:
      #   1. Instantiate the Function (so it can hold state for backward).
      #   2. Call its `forward(*inputs)` to compute the output Tensor.
      #   3. If any input has `requires_grad`, mark the output the same
      #      way and wire `output.grad_fn = self` so `Tensor#backward` can
      #      find us later.
      #
      # Returns the output Tensor.
      def self.apply(*inputs)
        fn = new

        # Stash the parent TENSORS so backward() can distribute grads
        # back to them.  Non-Tensor args (e.g. Pow's scalar exponent)
        # are dropped here — they don't participate in autograd and
        # `build_topo` in Tensor#backward would crash trying to read
        # `grad_fn` on a non-Tensor.  Subclasses that need a non-Tensor
        # arg can stash it in @saved_for_backward.
        #
        # Note: we save the actual Tensor objects; object identity
        # matters for shared-parameter scenarios like `Add(x, x)`
        # where the same Tensor appears twice — backward needs to
        # accumulate both gradient contributions into the same `.grad`.
        fn.parents = inputs.select { |i| i.is_a?(Tensor) }

        output = fn.forward(*inputs)

        # If ANY input tracks gradients, the output tracks them too — and
        # we record this Function as the source of the output, so backward
        # can walk back through us.
        needs_grad = inputs.any? { |t| t.is_a?(Tensor) && t.requires_grad }
        if needs_grad
          output.requires_grad = true
          output.grad_fn = fn
        end

        output
      end

      # Subclasses override this.  Compute the forward result.
      # @return [Tensor]
      def forward(*_inputs)
        raise NotImplementedError, "#{self.class} must implement #forward"
      end

      # Subclasses override this.  Given the gradient of the loss w.r.t.
      # this Function's output, return an Array of gradients w.r.t. each
      # input tensor (in the same order as `parents`).  Return `nil` in
      # a slot for any input that doesn't need a gradient.
      # @return [Array<Tensor, nil>]
      def backward(_output_grad)
        raise NotImplementedError, "#{self.class} must implement #backward"
      end

      # Friendly default for pp / inspect.
      def inspect
        "#<#{self.class.name} parents=#{@parents.length}>"
      end
    end

    # ========================================================================
    # Identity — the simplest possible Function subclass.
    #
    # forward(x)  → x' where x' is a fresh Tensor with the same data
    # backward(g) → [g]   (the upstream gradient passes through unchanged)
    #
    # Used by the autograd test suite to exercise the apply() / backward()
    # machinery without bringing in any op-specific math.  The real ops
    # land in PR #6 (ops.rb).
    # ========================================================================
    class Identity < Function
      def forward(x)
        # Return a NEW Tensor wrapping a copy of x's data, so the
        # autograd graph has a distinct node — pointer-equality
        # (`y.equal?(x)`) is false even though `y == x` is true.
        Tensor.new(x.to_a, shape: x.shape, dtype: x.dtype)
      end

      def backward(output_grad)
        # Identity's derivative is 1, so the gradient passes through
        # unchanged.  Wrap as an Array so apply() can distribute it to
        # parents[0] uniformly.
        [output_grad]
      end
    end
  end
end
