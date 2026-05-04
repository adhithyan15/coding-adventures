# frozen_string_literal: true

module CodingAdventures
  module InterpreterIr
    module Types
      CONCRETE = %w[u8 u16 u32 u64 bool str nil void].freeze
      DYNAMIC = "any"
      POLYMORPHIC = "polymorphic"

      def self.ref?(type_hint)
        type_hint.start_with?("ref<") && type_hint.end_with?(">")
      end

      def self.unwrap_ref(type_hint)
        return nil unless ref?(type_hint)

        type_hint[4...-1]
      end

      def self.ref(inner)
        "ref<#{inner}>"
      end

      def self.concrete?(type_hint)
        CONCRETE.include?(type_hint) || ref?(type_hint)
      end
    end

    module Opcodes
      ARITHMETIC = %w[add sub mul div mod neg].freeze
      BITWISE = %w[and or xor not shl shr].freeze
      CMP = %w[cmp_eq cmp_ne cmp_lt cmp_le cmp_gt cmp_ge].freeze
      BRANCH = %w[jmp jmp_if_true jmp_if_false].freeze
      CONTROL = %w[label ret ret_void].freeze
      MEMORY = %w[load_reg store_reg load_mem store_mem].freeze
      CALL = %w[call call_builtin].freeze
      IO = %w[io_in io_out].freeze
      COERCION = %w[cast type_assert].freeze
      HEAP = %w[alloc box unbox field_load field_store is_null safepoint].freeze
      VALUE = (ARITHMETIC + BITWISE + CMP + %w[const load_reg load_mem call call_builtin io_in cast alloc box unbox field_load is_null tetrad.move move]).freeze
      SIDE_EFFECT = (BRANCH + CONTROL + %w[store_reg store_mem io_out type_assert field_store safepoint]).freeze
      ALL = (VALUE + SIDE_EFFECT + MEMORY + CALL + IO + COERCION + HEAP).uniq.freeze
    end
  end
end
