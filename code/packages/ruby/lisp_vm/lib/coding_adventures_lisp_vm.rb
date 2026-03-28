# frozen_string_literal: true

require "coding_adventures_virtual_machine"
require "coding_adventures_garbage_collector"

require_relative "coding_adventures/lisp_vm/version"
require_relative "coding_adventures/lisp_vm/opcodes"
require_relative "coding_adventures/lisp_vm/vm"

module CodingAdventures
  module LispVm
    # NIL sentinel — Lisp's empty list / false value
    NIL = LispOp::NIL_SENTINEL
  end
end
