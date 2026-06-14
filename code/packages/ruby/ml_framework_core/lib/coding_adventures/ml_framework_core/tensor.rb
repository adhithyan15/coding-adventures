# frozen_string_literal: true

# Tensor — N-dimensional float array with shape, the universal currency
# of every deep-learning library.
# ============================================================================
#
# # Why this exists
#
# A Tensor is what you'd get if you bolted a `shape` onto a flat Ruby Array
# of Floats.  That's the whole idea.  The shape lets you treat the same
# underlying bytes as a 2x3 matrix, a 6-vector, a 1x6 row, a 6x1 column —
# whatever the math wants.
#
# # Storage model
#
# - `@data`  — flat Array of Float, length = `numel` = product(shape).
# - `@shape` — Array of positive Integer; can be empty for a 0-d scalar.
# - `@dtype` — Symbol; only :f32 supported in v0.1 (matches matrix-cpu).
#
# We use Ruby's native Float (which is f64) for the in-memory representation
# even though `@dtype` is :f32.  That's deliberate:
#   - Ruby has no f32 primitive; using f64 avoids the round-trip cost of
#     packing/unpacking for every elementwise op.
#   - The lossy step happens at the Rust dispatch boundary (PR #6), where
#     we hex-encode each f64 into 4 bytes of f32 before sending it across.
#   - For pure-Ruby v0.1, all storage is f64 and no precision is lost.
#
# # Autograd-prep slots (this PR sets them up; PR #5 uses them)
#
# - `@requires_grad` — if true, ops that consume this tensor will build an
#   autograd graph.  Just a boolean here; the actual graph machinery
#   lives in PR #5 (autograd.rb).
# - `@grad`          — Tensor of same shape, accumulated gradient.  nil
#   until backward() runs.
# - `@grad_fn`       — the Function (subclass of autograd.rb's Function)
#   that produced this tensor; nil for leaf tensors.
#
# These are present so PR #5 can wire up autograd WITHOUT touching this
# file.  No autograd LOGIC happens here.
#
# # What's NOT here (deferred)
#
# - Broadcasting:  v0.1 requires identical shapes for binary ops.  Adding
#   it now would couple Tensor to the shape-broadcasting algorithm; we
#   pull it in when ops dispatch lands (PR #6).
# - Indexing (`t[1, 2]`, slicing):  the Python version has a 50-line
#   `__getitem__` with integer / slice / list / tuple handling.  Adding
#   it here would double the file size for no autograd payoff; deferred
#   to a later PR.
# - Reductions (`sum`, `mean`):  these will be Function subclasses in
#   PR #6, not Tensor methods.

require "coding_adventures/ml_framework_core/version"

module CodingAdventures
  module MLFrameworkCore
    class Tensor
      # ----------------------------------------------------------------------
      # Construction
      # ----------------------------------------------------------------------

      # @param data [Array<Numeric>, Array<Array<...>>, Numeric] a flat Array,
      #   a nested Array, or a single scalar.  Nested arrays are flattened
      #   and the shape is inferred from their nesting depth.
      # @param shape [Array<Integer>, nil] explicit shape, or nil to infer.
      # @param dtype [Symbol] only :f32 is supported in v0.1.
      # @param requires_grad [Boolean] whether to track this tensor for autograd.
      def initialize(data, shape: nil, dtype: :f32, requires_grad: false)
        raise ArgumentError, "dtype #{dtype.inspect} not supported; v0.1 supports :f32 only" unless dtype == :f32

        # Two construction paths:
        #
        # 1. `Tensor.new(nested_or_flat_array)` — infer shape from nesting,
        #    flatten the data.  This is the common case.
        # 2. `Tensor.new(flat_array, shape: [...])` — caller already has
        #    flat data and tells us the shape explicitly.  Used by all
        #    the factories below and by reshape/transpose.
        if shape.nil?
          @shape = self.class.infer_shape(data)
          @data  = self.class.flatten_data(data, @shape)
        else
          @shape = shape.map(&:to_i)
          flat   = data.is_a?(Array) ? data.flatten : [data]
          expected = @shape.reduce(1, :*)
          if flat.length != expected
            raise ArgumentError,
                  "data length #{flat.length} does not match shape #{@shape.inspect} (expected #{expected})"
          end
          @data = flat.map { |x| x.to_f }
        end

        @dtype         = dtype
        @requires_grad = !!requires_grad
        @grad          = nil
        @grad_fn       = nil
      end

      # ----------------------------------------------------------------------
      # Attribute readers — all the things users want to introspect
      # ----------------------------------------------------------------------

      attr_reader :shape, :dtype, :grad, :grad_fn
      attr_accessor :requires_grad

      def ndim
        @shape.length
      end

      def numel
        @shape.empty? ? 1 : @shape.reduce(1, :*)
      end

      # ----------------------------------------------------------------------
      # Conversions
      # ----------------------------------------------------------------------

      # Flat Array view of the underlying data.  Returns a copy so callers
      # can mutate without breaking us.
      def to_a
        @data.dup
      end

      # Nested Array matching `shape`.  Round-trip: `Tensor.new(t.to_nested_a) == t`.
      #
      # The recursion eats one shape dim per level: shape [2, 3] becomes
      # an array of 2 arrays of 3 floats.
      def to_nested_a
        return @data[0] if @shape.empty?    # 0-d scalar
        return @data.dup if @shape.length == 1

        chunk_size = @shape[1..].reduce(1, :*)
        sub_shape  = @shape[1..]
        Array.new(@shape[0]) do |i|
          slice = @data[(i * chunk_size)...((i + 1) * chunk_size)]
          self.class.new(slice, shape: sub_shape).to_nested_a
        end
      end

      # ----------------------------------------------------------------------
      # Equality + inspection
      # ----------------------------------------------------------------------

      # Two tensors are equal iff they have the same shape AND the same
      # element values.  Shape mismatch beats value mismatch in the check
      # so the failure message is more useful.
      def ==(other)
        return false unless other.is_a?(Tensor)
        return false unless other.shape == @shape

        @data == other.to_a
      end

      alias eql? ==

      def hash
        [@shape, @data].hash
      end

      # Compact representation suitable for irb / pp.
      def inspect
        preview = @data.first(6).map { |v| format("%g", v) }.join(", ")
        suffix  = @data.length > 6 ? ", ..." : ""
        "#<Tensor shape=#{@shape.inspect} dtype=#{@dtype} data=[#{preview}#{suffix}]>"
      end

      alias to_s inspect

      # ----------------------------------------------------------------------
      # Shape ops — produce new Tensors with the same underlying numbers
      # in a different shape.
      # ----------------------------------------------------------------------

      # Same elements, new shape.  Raises if numel doesn't match.
      def reshape(*new_shape)
        new_shape = new_shape.flatten.map(&:to_i)
        new_numel = new_shape.reduce(1, :*)
        if new_numel != numel
          raise ArgumentError,
                "reshape: new shape #{new_shape.inspect} has #{new_numel} elements but tensor has #{numel}"
        end
        self.class.new(@data.dup, shape: new_shape, dtype: @dtype)
      end

      # Collapse to 1-D.
      def flatten
        self.class.new(@data.dup, shape: [numel], dtype: @dtype)
      end

      # Permute axes.  2-D only in v0.1 (i.e. `transpose` with no args
      # swaps the two dims of a matrix; `transpose(1, 0)` does the same
      # explicitly).  Higher-rank transpose lands when ops dispatch does.
      def transpose(*perm)
        if perm.empty?
          raise ArgumentError, "transpose with no args is only defined for 2-D tensors (got #{@shape.inspect})" unless ndim == 2

          perm = [1, 0]
        elsif perm.length != ndim
          raise ArgumentError, "transpose perm length #{perm.length} != ndim #{ndim}"
        end

        # Sanity-check perm is a permutation of [0, ndim).
        unless perm.sort == (0...ndim).to_a
          raise ArgumentError, "transpose perm #{perm.inspect} is not a permutation of (0...#{ndim})"
        end

        # For v0.1 we only need the 2-D case; assert it and do the
        # straightforward transpose.  Higher-rank transpose involves
        # generic strided index math; deferred to ops dispatch.
        unless ndim == 2
          raise NotImplementedError, "transpose on rank-#{ndim} tensors not yet implemented (v0.1: 2-D only)"
        end

        rows, cols = @shape
        new_data = Array.new(rows * cols)
        (0...rows).each do |r|
          (0...cols).each do |c|
            new_data[c * rows + r] = @data[r * cols + c]
          end
        end
        self.class.new(new_data, shape: [cols, rows], dtype: @dtype)
      end

      # Drop size-1 dims.  With no axis, drops ALL size-1 dims.
      # With an integer axis, raises if that axis isn't size 1.
      def squeeze(axis = nil)
        if axis.nil?
          new_shape = @shape.reject { |d| d == 1 }
        else
          axis = axis.to_i
          axis += ndim if axis.negative?
          raise IndexError, "squeeze axis #{axis} out of range [-#{ndim}, #{ndim})" if axis.negative? || axis >= ndim
          raise ArgumentError, "cannot squeeze axis #{axis} of size #{@shape[axis]}" unless @shape[axis] == 1

          new_shape = @shape.dup
          new_shape.delete_at(axis)
        end
        # Numel didn't change → can reuse data directly.
        self.class.new(@data.dup, shape: new_shape, dtype: @dtype)
      end

      # Insert a size-1 axis at position `axis`.  Negative indices count
      # from the end (PyTorch convention).
      def unsqueeze(axis)
        axis = axis.to_i
        # Allow axis == ndim (insert at end); range is [-ndim-1, ndim].
        axis += ndim + 1 if axis.negative?
        raise IndexError, "unsqueeze axis #{axis} out of range [-#{ndim + 1}, #{ndim + 1}]" if axis.negative? || axis > ndim

        new_shape = @shape.dup
        new_shape.insert(axis, 1)
        self.class.new(@data.dup, shape: new_shape, dtype: @dtype)
      end

      # ----------------------------------------------------------------------
      # Operator overloads — element-wise, same shape only (v0.1)
      # ----------------------------------------------------------------------

      def +(other)
        binary_op(other) { |a, b| a + b }
      end

      def -(other)
        binary_op(other) { |a, b| a - b }
      end

      def *(other)
        binary_op(other) { |a, b| a * b }
      end

      def /(other)
        binary_op(other) { |a, b| a / b.to_f }
      end

      def **(other)
        binary_op(other) { |a, b| a**b }
      end

      def -@
        self.class.new(@data.map { |v| -v }, shape: @shape, dtype: @dtype)
      end

      # ----------------------------------------------------------------------
      # Class-level factory methods
      # ----------------------------------------------------------------------

      class << self
        # @return [Tensor] tensor of zeros with the given shape.
        def zeros(*shape)
          shape = shape.flatten.map(&:to_i)
          new(Array.new(shape.reduce(1, :*), 0.0), shape: shape)
        end

        def ones(*shape)
          shape = shape.flatten.map(&:to_i)
          new(Array.new(shape.reduce(1, :*), 1.0), shape: shape)
        end

        # @param shape [Array<Integer>] target shape (as a single Array arg)
        # @param fill_value [Numeric] value to fill with
        def full(shape, fill_value)
          shape = Array(shape).map(&:to_i)
          fv    = fill_value.to_f
          new(Array.new(shape.reduce(1, :*), fv), shape: shape)
        end

        # 2-D "identity-like" tensor.  When n == m this is the identity
        # matrix; for rectangular shapes it's the diagonal-of-ones
        # rectangle that NumPy / PyTorch eye() return.
        def eye(n, m = nil)
          m ||= n
          data = Array.new(n * m, 0.0)
          [n, m].min.times do |i|
            data[i * m + i] = 1.0
          end
          new(data, shape: [n, m])
        end

        # arange(stop) or arange(start, stop) or arange(start, stop, step).
        # Mirrors Python's range + NumPy's arange semantics: stop is
        # EXCLUSIVE.
        def arange(*args)
          start, stop, step =
            case args.length
            when 1 then [0, args[0], 1]
            when 2 then [args[0], args[1], 1]
            when 3 then [args[0], args[1], args[2]]
            else raise ArgumentError, "arange: wrong number of arguments (#{args.length} for 1..3)"
            end
          raise ArgumentError, "arange step cannot be zero" if step.zero?
          # Defence-in-depth: Float::INFINITY for stop would loop forever
          # and grow the result Array until OOM; NaN would silently produce
          # an empty Array (confusing).  Reject both up-front.
          [start, stop, step].each do |v|
            if v.respond_to?(:finite?) && !v.finite?
              raise ArgumentError, "arange bounds must be finite, got #{v.inspect}"
            end
          end

          data = []
          x = start.to_f
          if step.positive?
            while x < stop
              data << x
              x += step
            end
          else
            while x > stop
              data << x
              x += step
            end
          end
          new(data, shape: [data.length])
        end

        # Convenience alias for `Tensor.new(nested_array)`.
        def from_array(nested)
          new(nested)
        end

        # Standard-normal samples via Box-Muller.
        #
        # Why Box-Muller, not Ruby's `Random#rand`?
        #   `Random#rand` is uniform on [0, 1); we want N(0, 1).  Box-Muller
        #   transforms two uniforms into two normals via
        #     z0 = sqrt(-2 ln u1) * cos(2π u2)
        #     z1 = sqrt(-2 ln u1) * sin(2π u2)
        #   It's a textbook two-line algorithm; no need to pull in a
        #   distribution library.
        #
        # @param seed [Integer, nil] if non-nil, deterministic output.
        def randn(*shape, seed: nil)
          shape = shape.flatten.map(&:to_i)
          rng   = seed.nil? ? Random.new : Random.new(seed)
          numel = shape.reduce(1, :*)
          data  = Array.new(numel)
          i = 0
          while i < numel
            u1 = rng.rand
            u1 = Float::MIN if u1.zero?    # ln(0) is -∞; nudge into the domain
            u2 = rng.rand
            mag = Math.sqrt(-2.0 * Math.log(u1))
            data[i] = mag * Math.cos(2.0 * Math::PI * u2)
            data[i + 1] = mag * Math.sin(2.0 * Math::PI * u2) if i + 1 < numel
            i += 2
          end
          new(data, shape: shape)
        end

        # ====================================================================
        # Internal helpers used by `initialize` — exposed as class methods
        # so they're testable in isolation.
        # ====================================================================

        # Infer the shape of an arbitrary-depth nested Array.  Validates
        # that every sub-array at the same depth has the same length
        # (i.e. the structure is rectangular, not ragged).
        def infer_shape(data)
          return [] unless data.is_a?(Array)
          return [0] if data.empty?

          shape = []
          probe = data
          while probe.is_a?(Array)
            shape << probe.length
            probe = probe.first
          end

          # Validate rectangularity: walk every element at depth k and
          # confirm it has the expected length.
          validate_rectangular(data, shape, 0)

          shape
        end

        def validate_rectangular(node, shape, depth)
          return if depth >= shape.length

          unless node.is_a?(Array) && node.length == shape[depth]
            raise ArgumentError, "ragged nested array: expected length #{shape[depth]} at depth #{depth}"
          end

          node.each { |child| validate_rectangular(child, shape, depth + 1) }
        end

        # Flatten arbitrarily-nested data into a 1-D Array<Float>, asserting
        # length matches the inferred shape's numel.
        def flatten_data(data, shape)
          flat = data.is_a?(Array) ? data.flatten : [data]
          expected = shape.empty? ? 1 : shape.reduce(1, :*)
          if flat.length != expected
            raise ArgumentError,
                  "data length #{flat.length} does not match inferred shape #{shape.inspect} (numel #{expected})"
          end
          flat.map { |x| x.to_f }
        end
      end

      private

      # Element-wise binary op against another Tensor of the same shape, or
      # against a scalar (broadcasts to every element).  No NumPy-style
      # broadcasting in v0.1.
      def binary_op(other)
        if other.is_a?(Tensor)
          unless other.shape == @shape
            raise ArgumentError,
                  "shape mismatch: #{@shape.inspect} vs #{other.shape.inspect} (broadcasting not in v0.1)"
          end
          new_data = @data.each_with_index.map { |a, i| yield(a, other.to_a[i]) }
          self.class.new(new_data, shape: @shape, dtype: @dtype)
        elsif other.is_a?(Numeric)
          b = other.to_f
          new_data = @data.map { |a| yield(a, b) }
          self.class.new(new_data, shape: @shape, dtype: @dtype)
        else
          raise TypeError, "cannot combine Tensor with #{other.class}"
        end
      end
    end
  end
end
