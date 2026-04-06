# frozen_string_literal: true

# ==========================================================================
# coding_adventures_register_vm
# ==========================================================================
#
# Root require file for the register_vm gem. Loading this file makes the
# entire public API available under the CodingAdventures::RegisterVM namespace.
#
# Load order matters in Ruby: each file may reference constants defined in
# previously-required files. The order here follows the dependency graph:
#
#   version    (no deps)
#     opcodes  (no deps)
#     types    (no deps)
#     feedback (depends on types: VMObject, VMFunction, UNDEFINED)
#     scope    (depends on types: Context, UNDEFINED, VMError)
#     interpreter (depends on all of the above)
#
require_relative "coding_adventures/register_vm/version"
require_relative "coding_adventures/register_vm/opcodes"
require_relative "coding_adventures/register_vm/types"
require_relative "coding_adventures/register_vm/feedback"
require_relative "coding_adventures/register_vm/scope"
require_relative "coding_adventures/register_vm/interpreter"

module CodingAdventures
  module RegisterVM
    # Convenience wrapper: create a fresh interpreter and execute `code`.
    #
    # @param code [CodeObject]
    # @return [VMResult]
    def self.execute(code)
      Interpreter.new.execute(code)
    end

    # Convenience wrapper: create a fresh interpreter and execute `code`,
    # returning a trace of every instruction executed.
    #
    # @param code [CodeObject]
    # @return [Array<TraceStep>]
    def self.execute_with_trace(code)
      Interpreter.new.execute_with_trace(code)
    end
  end
end
