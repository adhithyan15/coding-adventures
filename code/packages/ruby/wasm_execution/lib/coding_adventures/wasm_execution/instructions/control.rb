# frozen_string_literal: true

# ==========================================================================
# Control Flow Instruction Handlers for WASM
# ==========================================================================
#
# Implements structured control flow: block, loop, if/else/end, br,
# br_if, br_table, return, call, call_indirect, unreachable, nop.
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module Control
        module_function

        # Resolve a block type to its result arity (0 or 1 in WASM 1.0).
        def block_arity(block_type, _func_types)
          return 0 if block_type.nil? || block_type == 0x40
          return 1 if [0x7F, 0x7E, 0x7D, 0x7C].include?(block_type)
          0
        end

        # Execute a branch to the Nth label from the top of the label stack.
        def execute_branch(vm, ctx, label_index)
          label_stack_index = ctx[:label_stack].length - 1 - label_index
          raise TrapError, "branch target #{label_index} out of range" if label_stack_index < 0

          label = ctx[:label_stack][label_stack_index]

          # For loops, branch carries 0 values (WASM 1.0).
          # For blocks, branch carries arity result values.
          arity = label[:is_loop] ? 0 : label[:arity]

          # Save result values from the top of the stack.
          results = []
          arity.times { results.unshift(vm.pop_typed) }

          # Unwind the value stack to the label's recorded height.
          vm.pop_typed while vm.typed_stack.length > label[:stack_height]

          # Push results back.
          results.each { |v| vm.push_typed(v) }

          # Pop labels down to and including the target.
          ctx[:label_stack] = ctx[:label_stack][0...label_stack_index]

          # Jump to the label's target PC.
          vm.jump_to(label[:target_pc])
        end

        # Call a function by index (handles both host and module functions).
        def call_function(vm, code, ctx, func_index)
          func_type = ctx[:func_types][func_index]
          raise TrapError, "undefined function #{func_index}" unless func_type

          # Pop arguments from the stack (in reverse order).
          args = []
          func_type.params.length.times { args.unshift(vm.pop_typed) }

          # Check if this is a host function.
          host_func = ctx[:host_functions][func_index]
          if host_func
            results = host_func.call(args)
            results.each { |r| vm.push_typed(r) }
            vm.advance_pc
            return
          end

          # Module-defined function --- set up a new frame.
          body = ctx[:func_bodies][func_index]
          raise TrapError, "no body for function #{func_index}" unless body

          # Save the caller's state.
          ctx[:saved_frames].push({
            locals: ctx[:typed_locals].dup,
            label_stack: ctx[:label_stack].dup,
            stack_height: vm.typed_stack.length,
            control_flow_map: ctx[:control_flow_map],
            code: code,
            return_pc: vm.pc + 1,
            return_arity: func_type.results.length
          })

          # Initialize the callee's locals.
          ctx[:typed_locals] = args + body.locals.map { |t| WasmExecution.default_value(t) }

          # Clear label stack, build control flow map for callee.
          ctx[:label_stack] = []

          decoded = Decoder.decode_function_body(body)
          ctx[:control_flow_map] = Decoder.build_control_flow_map(decoded)

          # Hand control back to the engine so it can swap code objects cleanly.
          ctx[:pending_code] = CodingAdventures::VirtualMachine::CodeObject.new(
            instructions: Decoder.to_vm_instructions(decoded),
            constants: [],
            names: []
          )
          ctx[:returned] = false
          vm.jump_to(0)
          vm.halted = true
        end

        def register(vm)
          # 0x00: unreachable
          vm.register_context_opcode(0x00, ->(_vm, _instr, _code, _ctx) {
            raise TrapError, "unreachable instruction executed"
          })

          # 0x01: nop
          vm.register_context_opcode(0x01, ->(vm, _instr, _code, _ctx) {
            vm.advance_pc; "nop"
          })

          # 0x02: block
          vm.register_context_opcode(0x02, ->(vm, instr, _code, ctx) {
            block_type = instr.operand
            arity = Control.block_arity(block_type, ctx[:func_types])
            target = ctx[:control_flow_map][vm.pc]
            end_pc = target ? target.end_pc : vm.pc + 1

            ctx[:label_stack].push({
              arity: arity,
              target_pc: end_pc,
              stack_height: vm.typed_stack.length,
              is_loop: false
            })

            vm.advance_pc; "block"
          })

          # 0x03: loop
          vm.register_context_opcode(0x03, ->(vm, instr, _code, ctx) {
            block_type = instr.operand
            arity = Control.block_arity(block_type, ctx[:func_types])

            ctx[:label_stack].push({
              arity: arity,
              target_pc: vm.pc,  # loops branch backward!
              stack_height: vm.typed_stack.length,
              is_loop: true
            })

            vm.advance_pc; "loop"
          })

          # 0x04: if
          vm.register_context_opcode(0x04, ->(vm, instr, _code, ctx) {
            block_type = instr.operand
            arity = Control.block_arity(block_type, ctx[:func_types])
            condition = vm.pop_typed.value

            target = ctx[:control_flow_map][vm.pc]
            end_pc = target ? target.end_pc : vm.pc + 1
            else_pc = target ? target.else_pc : nil

            ctx[:label_stack].push({
              arity: arity,
              target_pc: end_pc,
              stack_height: vm.typed_stack.length,
              is_loop: false
            })

            if condition != 0
              vm.advance_pc
            else
              vm.jump_to(else_pc ? else_pc + 1 : end_pc)
            end
            "if"
          })

          # 0x05: else
          vm.register_context_opcode(0x05, ->(vm, _instr, _code, ctx) {
            label = ctx[:label_stack].last
            vm.jump_to(label[:target_pc])
            "else"
          })

          # 0x0B: end
          vm.register_context_opcode(0x0B, ->(vm, _instr, _code, ctx) {
            if ctx[:label_stack].length > 0
              ctx[:label_stack].pop
              vm.advance_pc
              "end (block)"
            else
              # Branches can legally jump to a block's `end` instruction after
              # its label has already been popped. Only the final `end` of the
              # function should stop execution.
              if vm.pc >= ctx[:current_instructions].length - 1
                ctx[:returned] = true
                vm.halted = true
                "end (function)"
              else
                vm.advance_pc
                "end (continue)"
              end
            end
          })

          # 0x0C: br
          vm.register_context_opcode(0x0C, ->(vm, instr, _code, ctx) {
            label_index = instr.operand
            Control.execute_branch(vm, ctx, label_index)
            "br"
          })

          # 0x0D: br_if
          vm.register_context_opcode(0x0D, ->(vm, instr, _code, ctx) {
            label_index = instr.operand
            condition = vm.pop_typed.value

            if condition != 0
              Control.execute_branch(vm, ctx, label_index)
            else
              vm.advance_pc
            end
            "br_if"
          })

          # 0x0E: br_table
          vm.register_context_opcode(0x0E, ->(vm, instr, _code, ctx) {
            table = instr.operand
            labels = table[:labels]
            default_label = table[:default_label]
            index = vm.pop_typed.value

            target_label = (index >= 0 && index < labels.length) ? labels[index] : default_label
            Control.execute_branch(vm, ctx, target_label)
            "br_table"
          })

          # 0x0F: return
          vm.register_context_opcode(0x0F, ->(vm, _instr, _code, ctx) {
            ctx[:returned] = true
            vm.halted = true
            "return"
          })

          # 0x10: call
          vm.register_context_opcode(0x10, ->(vm, instr, code, ctx) {
            func_index = instr.operand
            Control.call_function(vm, code, ctx, func_index)
            "call"
          })

          # 0x11: call_indirect
          vm.register_context_opcode(0x11, ->(vm, instr, _code, ctx) {
            operand = instr.operand
            type_idx = operand[:typeidx] || operand[:typeIdx]
            table_idx = operand[:tableidx] || operand[:tableIdx] || 0
            elem_index = vm.pop_typed.value

            table = ctx[:tables][table_idx]
            raise TrapError, "undefined table" unless table

            func_index = table.get(elem_index)
            raise TrapError, "uninitialized table element" if func_index.nil?

            expected = ctx[:func_types][type_idx]
            actual = ctx[:func_types][func_index]
            raise TrapError, "undefined type" unless expected && actual

            unless expected.params == actual.params && expected.results == actual.results
              raise TrapError, "indirect call type mismatch"
            end

            Control.call_function(vm, ctx, func_index)
            "call_indirect"
          })
        end
      end
    end
  end
end
