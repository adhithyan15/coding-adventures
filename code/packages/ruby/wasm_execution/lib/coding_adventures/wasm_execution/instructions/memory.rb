# frozen_string_literal: true

# ==========================================================================
# Memory Instruction Handlers for WASM
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module Memory
        module_function

        def effective_addr(base, operand)
          WasmExecution.to_u32(base) + (operand.is_a?(Hash) ? operand[:offset] : 0)
        end

        def require_memory(ctx)
          raise TrapError, "no linear memory" if ctx[:memory].nil?
          ctx[:memory]
        end

        def register(vm) # rubocop:disable Metrics/MethodLength
          # ── Load Instructions ──────────────────────────────────────

          # 0x28: i32.load
          vm.register_context_opcode(0x28, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx)
            base = WasmExecution.as_i32(vm.pop_typed)
            addr = Memory.effective_addr(base, instr.operand)
            vm.push_typed(WasmExecution.i32(mem.load_i32(addr)))
            vm.advance_pc; "i32.load"
          })

          # 0x29: i64.load
          vm.register_context_opcode(0x29, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx)
            base = WasmExecution.as_i32(vm.pop_typed)
            addr = Memory.effective_addr(base, instr.operand)
            vm.push_typed(WasmExecution.i64(mem.load_i64(addr)))
            vm.advance_pc; "i64.load"
          })

          # 0x2A: f32.load
          vm.register_context_opcode(0x2A, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx)
            base = WasmExecution.as_i32(vm.pop_typed)
            addr = Memory.effective_addr(base, instr.operand)
            vm.push_typed(WasmExecution.f32(mem.load_f32(addr)))
            vm.advance_pc; "f32.load"
          })

          # 0x2B: f64.load
          vm.register_context_opcode(0x2B, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx)
            base = WasmExecution.as_i32(vm.pop_typed)
            addr = Memory.effective_addr(base, instr.operand)
            vm.push_typed(WasmExecution.f64(mem.load_f64(addr)))
            vm.advance_pc; "f64.load"
          })

          # 0x2C-0x2F: i32 narrow loads
          vm.register_context_opcode(0x2C, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(mem.load_i32_8s(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i32.load8_s"
          })
          vm.register_context_opcode(0x2D, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(mem.load_i32_8u(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i32.load8_u"
          })
          vm.register_context_opcode(0x2E, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(mem.load_i32_16s(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i32.load16_s"
          })
          vm.register_context_opcode(0x2F, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(mem.load_i32_16u(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i32.load16_u"
          })

          # 0x30-0x35: i64 narrow loads
          vm.register_context_opcode(0x30, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(mem.load_i64_8s(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i64.load8_s"
          })
          vm.register_context_opcode(0x31, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(mem.load_i64_8u(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i64.load8_u"
          })
          vm.register_context_opcode(0x32, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(mem.load_i64_16s(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i64.load16_s"
          })
          vm.register_context_opcode(0x33, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(mem.load_i64_16u(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i64.load16_u"
          })
          vm.register_context_opcode(0x34, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(mem.load_i64_32s(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i64.load32_s"
          })
          vm.register_context_opcode(0x35, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); base = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(mem.load_i64_32u(Memory.effective_addr(base, instr.operand))))
            vm.advance_pc; "i64.load32_u"
          })

          # ── Store Instructions ─────────────────────────────────────

          # 0x36: i32.store
          vm.register_context_opcode(0x36, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx)
            value = WasmExecution.as_i32(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_i32(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "i32.store"
          })

          vm.register_context_opcode(0x37, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); value = WasmExecution.as_i64(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_i64(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "i64.store"
          })

          vm.register_context_opcode(0x38, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); value = WasmExecution.as_f32(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_f32(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "f32.store"
          })

          vm.register_context_opcode(0x39, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); value = WasmExecution.as_f64(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_f64(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "f64.store"
          })

          # Narrow stores
          vm.register_context_opcode(0x3A, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); value = WasmExecution.as_i32(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_i32_8(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "i32.store8"
          })
          vm.register_context_opcode(0x3B, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); value = WasmExecution.as_i32(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_i32_16(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "i32.store16"
          })
          vm.register_context_opcode(0x3C, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); value = WasmExecution.as_i64(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_i64_8(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "i64.store8"
          })
          vm.register_context_opcode(0x3D, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); value = WasmExecution.as_i64(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_i64_16(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "i64.store16"
          })
          vm.register_context_opcode(0x3E, ->(vm, instr, _code, ctx) {
            mem = Memory.require_memory(ctx); value = WasmExecution.as_i64(vm.pop_typed)
            base = WasmExecution.as_i32(vm.pop_typed)
            mem.store_i64_32(Memory.effective_addr(base, instr.operand), value)
            vm.advance_pc; "i64.store32"
          })

          # 0x3F: memory.size
          vm.register_context_opcode(0x3F, ->(vm, _instr, _code, ctx) {
            mem = Memory.require_memory(ctx)
            vm.push_typed(WasmExecution.i32(mem.page_count))
            vm.advance_pc; "memory.size"
          })

          # 0x40: memory.grow
          vm.register_context_opcode(0x40, ->(vm, _instr, _code, ctx) {
            mem = Memory.require_memory(ctx)
            delta = WasmExecution.as_i32(vm.pop_typed)
            result = mem.grow(delta)
            vm.push_typed(WasmExecution.i32(result))
            vm.advance_pc; "memory.grow"
          })
        end
      end
    end
  end
end
