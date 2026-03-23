# frozen_string_literal: true

# ==========================================================================
# Starlark Builtins -- The 23 Built-in Functions
# ==========================================================================
#
# Starlark provides a small set of built-in functions that are always
# available without importing. These mirror a subset of Python's builtins.
#
# When Starlark code calls `len([1,2,3])`, the execution flow is:
#
#   1. LOAD_NAME "len"    -- the LOAD_NAME handler finds the builtin
#   2. LOAD_CONST [1,2,3] -- push the argument
#   3. CALL_FUNCTION 1    -- call with 1 argument
#
# The CALL_FUNCTION handler detects that the callable is a BuiltinFunction
# and calls its implementation lambda directly, pushing the result.
#
# Each builtin is registered as a BuiltinFunction with a name and a lambda.
# The lambda receives an array of arguments and returns a value.
#
# == Builtin List
#
#   print(*args)      -- Print values to output, returns None
#   len(x)            -- Length of string, list, dict, or tuple
#   type(x)           -- Type name as a string
#   bool(x)           -- Convert to boolean (Starlark truthiness)
#   int(x)            -- Convert to integer
#   float(x)          -- Convert to float
#   str(x)            -- Convert to string
#   list(x)           -- Convert to list (copies lists, splits strings)
#   dict(pairs)       -- Create dict from key-value pairs
#   tuple(x)          -- Convert to tuple (frozen array)
#   range(stop)       -- Generate integer sequence [0, stop)
#   range(start,stop) -- Generate integer sequence [start, stop)
#   sorted(x)         -- Return a new sorted list
#   reversed(x)       -- Return a new reversed list
#   enumerate(x)      -- Return list of [index, value] pairs
#   zip(a, b)         -- Zip two lists into pairs
#   min(x)            -- Minimum value in a collection
#   max(x)            -- Maximum value in a collection
#   abs(x)            -- Absolute value
#   all(x)            -- True if all elements are truthy
#   any(x)            -- True if any element is truthy
#   repr(x)           -- String representation with quotes
#   hasattr(obj, name)-- Check if object has attribute (dict key)
#   getattr(obj, name)-- Get attribute value (dict value)
# ==========================================================================

module CodingAdventures
  module StarlarkVM
    module Builtins
      # Register all 23 builtin functions with the given GenericVM.
      def self.register_all(vm)
        # ================================================================
        # print(*args) -> None
        # ================================================================
        #
        # Print values separated by spaces, appending a newline.
        # The output is captured in vm.output for later retrieval.
        #
        #   print("hello", "world")  =>  "hello world"
        #   print(42)                =>  "42"
        #   print()                  =>  ""
        vm.register_builtin("print", ->(args) {
          strs = args.map { |a| Handlers.starlark_repr(a) }
          line = strs.join(" ")
          vm.output.push(line)
          nil  # print returns None
        })

        # ================================================================
        # len(x) -> int
        # ================================================================
        #
        # Return the number of items in a collection or characters in a string.
        #
        #   len([1, 2, 3])      => 3
        #   len("hello")        => 5
        #   len({"a": 1})       => 1
        #   len(())             => 0
        vm.register_builtin("len", ->(args) {
          obj = args[0]
          case obj
          when String, Array, Hash
            obj.length
          else
            raise "TypeError: object of type '#{obj.class}' has no len()"
          end
        })

        # ================================================================
        # type(x) -> str
        # ================================================================
        #
        # Return the Starlark type name of a value.
        #
        #   type(42)       => "int"
        #   type(3.14)     => "float"
        #   type("hi")     => "string"
        #   type([1])      => "list"
        #   type({})       => "dict"
        #   type(True)     => "bool"
        #   type(None)     => "NoneType"
        vm.register_builtin("type", ->(args) {
          val = args[0]
          case val
          when nil then "NoneType"
          when true, false then "bool"
          when Integer then "int"
          when Float then "float"
          when String then "string"
          when Array then "list"
          when Hash then "dict"
          when StarlarkFunction then "function"
          when CodingAdventures::VirtualMachine::BuiltinFunction then "builtin_function"
          else val.class.to_s
          end
        })

        # ================================================================
        # bool(x) -> bool
        # ================================================================
        #
        # Convert a value to boolean using Starlark truthiness rules.
        #
        #   bool(0)    => False
        #   bool(1)    => True
        #   bool("")   => False
        #   bool("x")  => True
        #   bool([])   => False
        #   bool(None) => False
        vm.register_builtin("bool", ->(args) {
          Handlers.starlark_truthy?(args[0])
        })

        # ================================================================
        # int(x) -> int
        # ================================================================
        #
        # Convert a value to integer.
        #
        #   int("42")   => 42
        #   int(3.7)    => 3
        #   int(True)   => 1
        #   int(False)  => 0
        vm.register_builtin("int", ->(args) {
          val = args[0]
          case val
          when Integer then val
          when Float then val.to_i
          when String then Integer(val)
          when true then 1
          when false then 0
          else
            raise "TypeError: int() argument must be a string or number"
          end
        })

        # ================================================================
        # float(x) -> float
        # ================================================================
        #
        # Convert a value to floating-point number.
        #
        #   float("3.14") => 3.14
        #   float(42)     => 42.0
        vm.register_builtin("float", ->(args) {
          val = args[0]
          case val
          when Float then val
          when Integer then val.to_f
          when String then Float(val)
          when true then 1.0
          when false then 0.0
          else
            raise "TypeError: float() argument must be a string or number"
          end
        })

        # ================================================================
        # str(x) -> str
        # ================================================================
        #
        # Convert any value to its string representation.
        #
        #   str(42)        => "42"
        #   str(True)      => "True"
        #   str(None)      => "None"
        #   str([1, 2])    => "[1, 2]"
        vm.register_builtin("str", ->(args) {
          Handlers.starlark_repr(args[0])
        })

        # ================================================================
        # list(x) -> list
        # ================================================================
        #
        # Convert to a list. Copies existing lists, splits strings into
        # characters, and extracts dict keys.
        #
        #   list("abc")       => ["a", "b", "c"]
        #   list([1, 2, 3])   => [1, 2, 3]  (copy)
        #   list({"a": 1})    => ["a"]
        vm.register_builtin("list", ->(args) {
          val = args[0]
          case val
          when Array then val.dup
          when String then val.chars
          when Hash then val.keys
          else
            raise "TypeError: cannot convert '#{val.class}' to list"
          end
        })

        # ================================================================
        # dict(pairs) -> dict
        # ================================================================
        #
        # Create a dict from key-value pairs.
        #
        #   dict([["a", 1], ["b", 2]])  => {"a": 1, "b": 2}
        #   dict()                       => {}
        vm.register_builtin("dict", ->(args) {
          if args.empty?
            {}
          else
            val = args[0]
            case val
            when Hash then val.dup
            when Array
              result = {}
              val.each { |pair| result[pair[0]] = pair[1] }
              result
            else
              raise "TypeError: cannot convert '#{val.class}' to dict"
            end
          end
        })

        # ================================================================
        # tuple(x) -> tuple
        # ================================================================
        #
        # Convert to a tuple (immutable list). In Ruby, we represent
        # tuples as regular arrays since Ruby doesn't have a tuple type.
        #
        #   tuple([1, 2, 3])  => (1, 2, 3)
        #   tuple("abc")      => ("a", "b", "c")
        vm.register_builtin("tuple", ->(args) {
          val = args[0]
          case val
          when Array then val.dup
          when String then val.chars
          else
            raise "TypeError: cannot convert '#{val.class}' to tuple"
          end
        })

        # ================================================================
        # range([start,] stop [, step]) -> list
        # ================================================================
        #
        # Generate a sequence of integers. Unlike Python's lazy range(),
        # Starlark's range() returns a concrete list.
        #
        #   range(5)        => [0, 1, 2, 3, 4]
        #   range(2, 5)     => [2, 3, 4]
        #   range(0, 10, 2) => [0, 2, 4, 6, 8]
        vm.register_builtin("range", ->(args) {
          case args.length
          when 1
            (0...args[0]).to_a
          when 2
            (args[0]...args[1]).to_a
          when 3
            start = args[0]
            stop = args[1]
            step = args[2]
            result = []
            if step > 0
              i = start
              while i < stop
                result << i
                i += step
              end
            elsif step < 0
              i = start
              while i > stop
                result << i
                i += step
              end
            else
              raise "ValueError: range() step argument must not be zero"
            end
            result
          else
            raise "TypeError: range() requires 1 to 3 arguments"
          end
        })

        # ================================================================
        # sorted(iterable) -> list
        # ================================================================
        #
        # Return a new sorted list from the items in an iterable.
        #
        #   sorted([3, 1, 2])  => [1, 2, 3]
        #   sorted("cab")      => ["a", "b", "c"]
        vm.register_builtin("sorted", ->(args) {
          val = args[0]
          items = case val
          when Array then val.dup
          when String then val.chars
          when Hash then val.keys
          else
            raise "TypeError: cannot sort '#{val.class}'"
          end
          items.sort
        })

        # ================================================================
        # reversed(iterable) -> list
        # ================================================================
        #
        # Return a new reversed list.
        #
        #   reversed([1, 2, 3]) => [3, 2, 1]
        vm.register_builtin("reversed", ->(args) {
          val = args[0]
          items = case val
          when Array then val.dup
          when String then val.chars
          else
            raise "TypeError: cannot reverse '#{val.class}'"
          end
          items.reverse
        })

        # ================================================================
        # enumerate(iterable) -> list of [index, value]
        # ================================================================
        #
        # Return a list of [index, value] pairs.
        #
        #   enumerate(["a", "b"]) => [[0, "a"], [1, "b"]]
        vm.register_builtin("enumerate", ->(args) {
          val = args[0]
          items = case val
          when Array then val
          when String then val.chars
          else
            raise "TypeError: cannot enumerate '#{val.class}'"
          end
          items.each_with_index.map { |item, i| [i, item] }
        })

        # ================================================================
        # zip(a, b, ...) -> list of tuples
        # ================================================================
        #
        # Zip multiple iterables into a list of tuples.
        #
        #   zip([1, 2], [3, 4]) => [[1, 3], [2, 4]]
        vm.register_builtin("zip", ->(args) {
          lists = args.map { |a|
            case a
            when Array then a
            when String then a.chars
            else raise "TypeError: cannot zip '#{a.class}'"
            end
          }
          return [] if lists.empty?
          min_len = lists.map(&:length).min
          (0...min_len).map { |i| lists.map { |l| l[i] } }
        })

        # ================================================================
        # min(iterable) or min(a, b, ...) -> value
        # ================================================================
        #
        # Return the smallest item.
        #
        #   min([3, 1, 2])  => 1
        #   min(3, 1, 2)    => 1
        vm.register_builtin("min", ->(args) {
          if args.length == 1 && args[0].is_a?(Array)
            args[0].min
          else
            args.min
          end
        })

        # ================================================================
        # max(iterable) or max(a, b, ...) -> value
        # ================================================================
        #
        # Return the largest item.
        #
        #   max([3, 1, 2])  => 3
        #   max(3, 1, 2)    => 3
        vm.register_builtin("max", ->(args) {
          if args.length == 1 && args[0].is_a?(Array)
            args[0].max
          else
            args.max
          end
        })

        # ================================================================
        # abs(x) -> number
        # ================================================================
        #
        # Return the absolute value of a number.
        #
        #   abs(-5)    => 5
        #   abs(3.14)  => 3.14
        vm.register_builtin("abs", ->(args) {
          args[0].abs
        })

        # ================================================================
        # all(iterable) -> bool
        # ================================================================
        #
        # Return True if all elements are truthy (or the iterable is empty).
        #
        #   all([1, 2, 3])      => True
        #   all([1, 0, 3])      => False
        #   all([])             => True
        vm.register_builtin("all", ->(args) {
          val = args[0]
          items = val.is_a?(Array) ? val : [val]
          items.all? { |item| Handlers.starlark_truthy?(item) }
        })

        # ================================================================
        # any(iterable) -> bool
        # ================================================================
        #
        # Return True if any element is truthy.
        #
        #   any([0, 0, 1])     => True
        #   any([0, 0, 0])     => False
        #   any([])            => False
        vm.register_builtin("any", ->(args) {
          val = args[0]
          items = val.is_a?(Array) ? val : [val]
          items.any? { |item| Handlers.starlark_truthy?(item) }
        })

        # ================================================================
        # repr(x) -> str
        # ================================================================
        #
        # Return a string representation with quotes around strings.
        #
        #   repr(42)      => "42"
        #   repr("hello") => "\"hello\""
        #   repr(None)    => "None"
        vm.register_builtin("repr", ->(args) {
          Handlers.starlark_repr_quoted(args[0])
        })

        # ================================================================
        # hasattr(obj, name) -> bool
        # ================================================================
        #
        # Check if an object (dict) has the given attribute (key).
        #
        #   hasattr({"x": 1}, "x") => True
        #   hasattr({"x": 1}, "y") => False
        vm.register_builtin("hasattr", ->(args) {
          obj = args[0]
          name = args[1]
          obj.is_a?(Hash) && obj.key?(name)
        })

        # ================================================================
        # getattr(obj, name [, default]) -> value
        # ================================================================
        #
        # Get an attribute (dict key) from an object, with optional default.
        #
        #   getattr({"x": 1}, "x")       => 1
        #   getattr({"x": 1}, "y", 42)   => 42
        vm.register_builtin("getattr", ->(args) {
          obj = args[0]
          name = args[1]
          default_val = args[2]
          if obj.is_a?(Hash) && obj.key?(name)
            obj[name]
          elsif args.length >= 3
            default_val
          else
            raise "AttributeError: object has no attribute '#{name}'"
          end
        })
      end
    end
  end
end
