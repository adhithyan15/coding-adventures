# frozen_string_literal: true

require "stringio"
require "set"
require "coding_adventures_interpreter_ir"
require_relative "builtins"
require_relative "errors"
require_relative "frame"
require_relative "metrics"

module CodingAdventures
  module VmCore
    class VMCore
      attr_reader :builtins, :memory, :io_ports, :output
      attr_accessor :profiler_enabled

      def initialize(max_frames: 64, opcodes: {}, builtins: nil, profiler_enabled: true,
        u8_wrap: false, type_mapper: nil, input: "")
        @max_frames = max_frames
        @opcode_overrides = opcodes
        @builtins = builtins || BuiltinRegistry.new
        @profiler_enabled = profiler_enabled
        @u8_wrap = u8_wrap
        @type_mapper = type_mapper || method(:default_type_mapper)
        @jit_handlers = {}
        @frames = []
        @module = nil
        @interrupted = false
        @memory = {}
        @io_ports = {}
        @input = input.bytes
        @output = StringIO.new
        @metrics_instrs = 0
        @metrics_frames = 0
        @metrics_jit_hits = 0
        @fn_call_counts = Hash.new(0)
        @branch_stats = Hash.new { |h, k| h[k] = {} }
        @loop_back_edges = Hash.new { |h, k| h[k] = Hash.new(0) }
        @coverage_mode = false
        @coverage = Hash.new { |h, k| h[k] = Set.new }
      end

      def execute(mod, fn: "main", args: [])
        @module = mod
        @frames = []
        @interrupted = false
        run_function(fn, args || [])
      end

      def execute_traced(mod, fn: "main", args: [])
        traces = []
        result = execute_with_trace(mod, fn, args || [], traces)
        [result, traces]
      end

      def metrics
        VMMetrics.new(
          function_call_counts: @fn_call_counts.dup,
          total_instructions_executed: @metrics_instrs,
          total_frames_pushed: @metrics_frames,
          total_jit_hits: @metrics_jit_hits,
          branch_stats: deep_branch_stats,
          loop_back_edge_counts: @loop_back_edges.transform_values(&:dup)
        )
      end

      def reset_metrics
        @metrics_instrs = 0
        @metrics_frames = 0
        @metrics_jit_hits = 0
        @fn_call_counts = Hash.new(0)
        @branch_stats = Hash.new { |h, k| h[k] = {} }
        @loop_back_edges = Hash.new { |h, k| h[k] = Hash.new(0) }
      end

      def register_builtin(name, fn = nil, &block)
        @builtins.register(name, fn, &block)
      end

      def register_jit_handler(fn_name, handler)
        @jit_handlers[fn_name] = handler
      end

      def unregister_jit_handler(fn_name)
        @jit_handlers.delete(fn_name)
      end

      def hot_functions(threshold = 100)
        @fn_call_counts.select { |_name, count| count >= threshold }.keys
      end

      def branch_profile(fn_name, source_ip)
        @branch_stats.dig(fn_name, source_ip)
      end

      def loop_iterations(fn_name)
        @loop_back_edges.fetch(fn_name, {}).dup
      end

      def enable_coverage
        @coverage_mode = true
      end

      def disable_coverage
        @coverage_mode = false
      end

      def coverage_data
        @coverage.transform_values { |set| set.to_a.sort.freeze }
      end

      def reset_coverage
        @coverage_mode = false
        @coverage = Hash.new { |h, k| h[k] = Set.new }
      end

      def interrupt
        @interrupted = true
      end

      private

      def execute_with_trace(mod, fn, args, traces)
        @module = mod
        run_function(fn, args, traces)
      end

      def run_function(fn_name, args, traces = nil)
        if (handler = @jit_handlers[fn_name])
          @metrics_jit_hits += 1
          return handler.call(args)
        end

        fn = @module.get_function(fn_name)
        raise KeyError, "function #{fn_name.inspect} not found in module" unless fn
        raise FrameOverflowError, "maximum frame depth #{@max_frames} exceeded" if @frames.length >= @max_frames

        frame = VMFrame.new(fn, args)
        @frames << frame
        @metrics_frames += 1
        @fn_call_counts[fn.name] += 1
        fn.call_count += 1

        begin
          loop do
            raise VMInterrupt, "execution interrupted" if @interrupted
            return nil if frame.ip >= fn.instructions.length

            instr_index = frame.ip
            instr = fn.instructions[instr_index]
            frame.ip += 1
            @metrics_instrs += 1
            @coverage[fn.name] << instr_index if @coverage_mode

            before = traces ? frame.registers.dup : nil
            result = dispatch(frame, instr, instr_index)
            after = traces ? frame.registers.dup : nil
            traces << {function: fn.name, ip: instr_index, instr: instr, before: before, after: after} if traces
            return result[:return] if result.is_a?(Hash) && result.key?(:return)
          end
        ensure
          @frames.pop
        end
      end

      def dispatch(frame, instr, instr_index)
        if (handler = @opcode_overrides[instr.op])
          return handler.call(self, frame, instr)
        end

        case instr.op
        when "const"
          write_observed(frame, instr, instr.srcs.first)
        when "move", "tetrad.move"
          write_observed(frame, instr, frame.resolve(instr.srcs.first))
        when "add", "sub", "mul", "div", "mod", "and", "or", "xor", "shl", "shr"
          lhs = frame.resolve(instr.srcs[0])
          rhs = frame.resolve(instr.srcs[1])
          write_observed(frame, instr, numeric_op(instr.op, lhs, rhs))
        when "neg"
          write_observed(frame, instr, wrap(-Integer(frame.resolve(instr.srcs[0]))))
        when "not"
          write_observed(frame, instr, ~Integer(frame.resolve(instr.srcs[0])))
        when "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"
          write_observed(frame, instr, compare(instr.op, frame.resolve(instr.srcs[0]), frame.resolve(instr.srcs[1])))
        when "cast"
          write_observed(frame, instr, cast(frame.resolve(instr.srcs[0]), instr.srcs[1]))
        when "type_assert"
          value = frame.resolve(instr.srcs[0])
          expected = instr.srcs[1]
          actual = default_type_mapper(value)
          raise VMError, "type assertion failed: expected #{expected}, got #{actual}" unless expected == actual
        when "label"
          nil
        when "jmp"
          target = frame.fn.label_index(instr.srcs.first)
          record_loop(frame.fn.name, instr_index, target)
          frame.ip = target
        when "jmp_if_true", "jmp_if_false"
          cond = truthy?(frame.resolve(instr.srcs[0]))
          taken = instr.op == "jmp_if_true" ? cond : !cond
          record_branch(frame.fn.name, instr_index, taken)
          if taken
            target = frame.fn.label_index(instr.srcs[1])
            record_loop(frame.fn.name, instr_index, target)
            frame.ip = target
          end
        when "ret"
          {return: frame.resolve(instr.srcs.first)}
        when "ret_void"
          {return: nil}
        when "call"
          fn_name = instr.srcs.first.to_s
          args = instr.srcs[1..].map { |src| frame.resolve(src) }
          write_observed(frame, instr, run_function(fn_name, args))
        when "call_builtin"
          name = instr.srcs.first.to_s
          args = instr.srcs[1..].map { |src| frame.resolve(src) }
          value = @builtins.call(name, args)
          write_observed(frame, instr, value) if instr.dest
          value
        when "load_reg"
          write_observed(frame, instr, frame.load_slot(Integer(frame.resolve(instr.srcs.first))))
        when "store_reg"
          index = Integer(frame.resolve(instr.srcs[0]))
          value = frame.resolve(instr.srcs[1])
          frame.store_slot(index, value)
        when "load_mem"
          write_observed(frame, instr, @memory.fetch(Integer(frame.resolve(instr.srcs.first)), 0))
        when "store_mem"
          @memory[Integer(frame.resolve(instr.srcs[0]))] = frame.resolve(instr.srcs[1])
        when "io_in"
          write_observed(frame, instr, @input.shift || 0)
        when "io_out"
          value = Integer(frame.resolve(instr.srcs.first)) & 0xFF
          @output << value.chr
          value
        else
          raise UnknownOpcodeError, "unknown opcode #{instr.op.inspect}"
        end
      end

      def write_observed(frame, instr, value)
        value = wrap(value) if @u8_wrap && instr.type_hint == "u8" && value.is_a?(Integer)
        frame.write(instr.dest, value)
        instr.record_observation(@type_mapper.call(value)) if @profiler_enabled && instr.dest
        value
      end

      def numeric_op(op, lhs, rhs)
        lhs = Integer(lhs)
        rhs = Integer(rhs)
        value = case op
        when "add" then lhs + rhs
        when "sub" then lhs - rhs
        when "mul" then lhs * rhs
        when "div"
          raise ZeroDivisionError, "division by zero" if rhs.zero?
          lhs / rhs
        when "mod"
          raise ZeroDivisionError, "modulo by zero" if rhs.zero?
          lhs % rhs
        when "and" then lhs & rhs
        when "or" then lhs | rhs
        when "xor" then lhs ^ rhs
        when "shl" then lhs << rhs
        when "shr" then lhs >> rhs
        end
        wrap(value)
      end

      def compare(op, lhs, rhs)
        case op
        when "cmp_eq" then lhs == rhs
        when "cmp_ne" then lhs != rhs
        when "cmp_lt" then lhs < rhs
        when "cmp_le" then lhs <= rhs
        when "cmp_gt" then lhs > rhs
        when "cmp_ge" then lhs >= rhs
        end
      end

      def cast(value, type)
        case type
        when "u8" then Integer(value) & 0xFF
        when "bool" then truthy?(value)
        when "str" then value.to_s
        else value
        end
      end

      def truthy?(value)
        !(value.nil? || value == false || value == 0)
      end

      def wrap(value)
        @u8_wrap && value.is_a?(Integer) ? (value & 0xFF) : value
      end

      def record_branch(fn_name, ip, taken)
        current = @branch_stats[fn_name][ip] || BranchStats.new
        @branch_stats[fn_name][ip] = current.record(taken)
      end

      def record_loop(fn_name, source_ip, target_ip)
        @loop_back_edges[fn_name][source_ip] += 1 if target_ip < source_ip
      end

      def deep_branch_stats
        @branch_stats.each_with_object({}) do |(fn, stats), out|
          out[fn] = stats.dup
        end
      end

      def default_type_mapper(value)
        case value
        when TrueClass, FalseClass then "bool"
        when Integer then "u64"
        when String then "str"
        when NilClass then "nil"
        else "any"
        end
      end
    end
  end
end
