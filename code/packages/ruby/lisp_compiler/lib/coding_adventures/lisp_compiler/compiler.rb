# frozen_string_literal: true

# ================================================================
# Lisp Compiler — Compiles Lisp ASTs to GenericVM Bytecode
# ================================================================
#
# Lisp has one of the simplest grammars of any language:
#
#   program   = { sexpr } ;
#   sexpr     = atom | list | quoted ;
#   atom      = NUMBER | SYMBOL | STRING ;
#   list      = LPAREN list_body RPAREN ;
#   list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;
#   quoted    = QUOTE sexpr ;
#
# But the grammar doesn't distinguish function calls from special forms —
# both look like lists. The compiler inspects the first element of each
# list to decide:
#
#   (define x 1)    → definition (STORE_NAME)
#   (lambda (n) n)  → closure creation (MAKE_CLOSURE)
#   (cond ...)      → conditional (JUMP_IF_FALSE)
#   (+ 1 2)         → arithmetic (ADD)
#   (print x)       → I/O (PRINT)
#   (f arg)         → function call (CALL_FUNCTION)
#
# This is "homoiconicity" — code and data share the same structure.
# The grammar can't tell them apart; the compiler assigns meaning.
#
# Special Forms (cannot be regular functions — control evaluation order)
# -----------------------------------------------------------------------
#   (define x expr)         — bind x to result of expr
#   (lambda (params) body)  — create a closure
#   (cond (p1 e1) ...)      — conditional: evaluate only the matching branch
#   (quote expr)            — return data without evaluating
#
# Tail Call Optimization
# ----------------------
# A call is in tail position when it is the last thing a function does.
# The compiler tracks tail position and emits TAIL_CALL instead of
# CALL_FUNCTION for tail-position calls. (The VM's TAIL_CALL handler
# is generic — this is a compiler-level optimization.)
# ================================================================

require "coding_adventures_bytecode_compiler"
require "coding_adventures_lisp_parser"
require "coding_adventures_lisp_vm"

module CodingAdventures
  module LispCompiler
    VM_MOD  = CodingAdventures::VirtualMachine
    LOP     = CodingAdventures::LispVm::LispOp
    NIL_VAL = CodingAdventures::LispVm::NIL

    # ================================================================
    # Compilation State
    # ================================================================
    #
    # The compiler is a simple recursive descent code generator. It
    # walks the AST and emits instructions into a mutable list.
    #
    # State is kept in a CompileContext struct:
    #   instructions — the bytecode being built
    #   constants    — constant pool (LOAD_CONST operands)
    #   names        — name pool (STORE_NAME/LOAD_NAME operands)
    #   in_function  — whether we are compiling a lambda body (affects
    #                  whether we use STORE_NAME or STORE_LOCAL, and
    #                  whether tail position is tracked)

    CompileContext = Struct.new(:instructions, :constants, :names, :in_function) do
      def emit(opcode, operand = 0)
        instructions << VM_MOD::Instruction.new(opcode: opcode, operand: operand)
        instructions.size - 1
      end

      def add_constant(val)
        idx = constants.index(val)
        return idx if idx
        constants << val
        constants.size - 1
      end

      def add_name(name)
        idx = names.index(name)
        return idx if idx
        names << name
        names.size - 1
      end

      def patch_jump(idx, target)
        instructions[idx] = VM_MOD::Instruction.new(
          opcode: instructions[idx].opcode,
          operand: target
        )
      end

      def to_code_object
        VM_MOD::CodeObject.new(
          instructions: instructions.dup,
          constants:    constants.dup,
          names:        names.dup
        )
      end
    end

    # Compile Lisp source code and return a CodeObject.
    #
    # @param source [String] Lisp source code
    # @return [CodingAdventures::VirtualMachine::CodeObject]
    def self.compile_lisp(source)
      ast = CodingAdventures::LispParser.parse(source)
      ctx = CompileContext.new([], [], [], false)
      compile_program(ast, ctx, tail: false)
      ctx.emit(LOP::HALT)
      ctx.to_code_object
    end

    # Compile Lisp source and run it immediately.
    # Returns the VM after execution (access .stack, .variables, .output).
    #
    # @param source [String]
    # @return [CodingAdventures::VirtualMachine::GenericVM]
    def self.run_lisp(source)
      code = compile_lisp(source)
      vm   = CodingAdventures::LispVm.create_lisp_vm
      vm.execute(code)
      vm
    end

    # ================================================================
    # AST Walking Helpers
    # ================================================================

    def self.rule_name(node)
      node.respond_to?(:rule_name) ? node.rule_name : nil
    end

    def self.children(node)
      node.respond_to?(:children) ? node.children : []
    end

    def self.token?(node)
      node.respond_to?(:type) && !node.respond_to?(:rule_name)
    end

    # Unwrap single-child wrapper nodes (sexpr → actual content)
    def self.unwrap(node)
      return node unless rule_name(node)
      loop do
        cs = children(node)
        break unless cs.size == 1
        break unless rule_name(cs[0])
        node = cs[0]
      end
      node
    end

    # ================================================================
    # Top-Level Compilation
    # ================================================================

    def self.compile_program(node, ctx, tail:)
      node = unwrap(node)
      case rule_name(node)
      when "program"
        sexprs = children(node).select { |c| rule_name(c) == "sexpr" }
        sexprs.each_with_index do |sexpr, i|
          is_last = i == sexprs.size - 1
          compile_sexpr(sexpr, ctx, tail: tail && is_last)
          ctx.emit(LOP::POP) unless is_last
        end
      else
        compile_sexpr(node, ctx, tail: tail)
      end
    end

    def self.compile_sexpr(node, ctx, tail:)
      node = unwrap(node)
      case rule_name(node)
      when "sexpr"   then compile_sexpr(children(node).first, ctx, tail: tail)
      when "atom"    then compile_atom(node, ctx)
      when "list"    then compile_list(node, ctx, tail: tail)
      when "quoted"  then compile_quoted(node, ctx)
      else
        # Bare token at this level
        compile_token(node, ctx) if token?(node)
      end
    end

    # ================================================================
    # Atoms
    # ================================================================

    def self.compile_atom(node, ctx)
      tok = children(node).first
      return unless tok

      type_str = tok.type.to_s
      case type_str
      when "NUMBER"
        val = tok.value.include?(".") ? tok.value.to_f : tok.value.to_i
        ctx.emit(LOP::LOAD_CONST, ctx.add_constant(val))
      when "STRING"
        # Strip surrounding quotes
        str = tok.value[1..-2]
        ctx.emit(LOP::LOAD_CONST, ctx.add_constant(str))
      when "SYMBOL"
        name = tok.value
        if name == "nil" || name == "()"
          ctx.emit(LOP::LOAD_NIL)
        elsif name == "t" || name == "true"
          ctx.emit(LOP::LOAD_TRUE)
        else
          ctx.emit(LOP::LOAD_NAME, ctx.add_name(name))
        end
      end
    end

    def self.compile_token(tok, ctx)
      type_str = tok.type.to_s
      case type_str
      when "NUMBER"
        val = tok.value.include?(".") ? tok.value.to_f : tok.value.to_i
        ctx.emit(LOP::LOAD_CONST, ctx.add_constant(val))
      when "SYMBOL"
        ctx.emit(LOP::LOAD_NAME, ctx.add_name(tok.value))
      end
    end

    # ================================================================
    # Lists — Dispatch on Head Symbol
    # ================================================================

    def self.compile_list(node, ctx, tail:)
      cs = children(node)
      # Filter: keep only sexpr nodes and non-paren tokens
      body_items = cs.select do |c|
        rule_name(c) == "list_body"
      end
      return ctx.emit(LOP::LOAD_NIL) if body_items.empty?

      list_body = body_items.first
      body_children = children(list_body).select do |c|
        rule_name(c) == "sexpr" || (token?(c) && c.type.to_s != "DOT")
      end
      return ctx.emit(LOP::LOAD_NIL) if body_children.empty?

      head = unwrap(body_children.first)
      rest = body_children[1..]

      # Get head symbol name if it's a symbol atom
      head_name = extract_symbol_name(head)

      case head_name
      when "define"  then compile_define(rest, ctx)
      when "lambda"  then compile_lambda(rest, ctx)
      when "cond"    then compile_cond(rest, ctx, tail: tail)
      when "quote"   then compile_quote_form(rest, ctx)
      when "cons"    then compile_builtin_2(rest, ctx, LOP::CONS)
      when "car"     then compile_builtin_1(rest, ctx, LOP::CAR)
      when "cdr"     then compile_builtin_1(rest, ctx, LOP::CDR)
      when "atom"    then compile_builtin_1(rest, ctx, LOP::IS_ATOM)
      when "nil?"    then compile_builtin_1(rest, ctx, LOP::IS_NIL)
      when "eq"      then compile_builtin_2(rest, ctx, LOP::CMP_EQ)
      when "+"       then compile_arith(rest, ctx, LOP::ADD)
      when "-"       then compile_arith(rest, ctx, LOP::SUB)
      when "*"       then compile_arith(rest, ctx, LOP::MUL)
      when "/"       then compile_arith(rest, ctx, LOP::DIV)
      when "<"       then compile_builtin_2(rest, ctx, LOP::CMP_LT)
      when ">"       then compile_builtin_2(rest, ctx, LOP::CMP_GT)
      when "print"   then compile_print(rest, ctx)
      else                compile_call(head, rest, ctx, tail: tail)
      end
    end

    def self.extract_symbol_name(node)
      node = unwrap(node)
      if rule_name(node) == "atom"
        tok = children(node).first
        tok&.type&.to_s == "SYMBOL" ? tok.value : nil
      elsif token?(node) && node.type.to_s == "SYMBOL"
        node.value
      end
    end

    # ================================================================
    # Special Forms
    # ================================================================

    def self.compile_define(args, ctx)
      return if args.empty?
      name_node = unwrap(args[0])
      sym_name  = extract_symbol_name(name_node)
      return unless sym_name

      if args.size >= 2
        compile_sexpr(args[1], ctx, tail: false)
      else
        ctx.emit(LOP::LOAD_NIL)
      end
      ctx.emit(LOP::STORE_NAME, ctx.add_name(sym_name))
      ctx.emit(LOP::LOAD_NIL)
    end

    def self.compile_lambda(args, ctx)
      # (lambda (params...) body)
      params = extract_param_names(args[0])
      body_sexpr = args[1]

      # Compile the lambda body into a separate CodeObject
      inner_ctx = CompileContext.new([], [], params.dup, true)
      if body_sexpr
        compile_sexpr(body_sexpr, inner_ctx, tail: true)
      else
        inner_ctx.emit(LOP::LOAD_NIL)
      end
      inner_ctx.emit(LOP::RETURN)
      fn_code = inner_ctx.to_code_object

      idx = ctx.add_constant(fn_code)
      ctx.emit(LOP::MAKE_CLOSURE, idx)
    end

    def self.extract_param_names(param_node)
      return [] unless param_node
      param_node = unwrap(param_node)
      # Params are a list: (a b c) or bare sexprs
      case rule_name(param_node)
      when "list"
        body_items = children(param_node).select { |c| rule_name(c) == "list_body" }
        return [] if body_items.empty?
        sexprs = children(body_items.first).select { |c| rule_name(c) == "sexpr" }
        sexprs.filter_map { |s| extract_symbol_name(s) }
      when "sexpr", "atom"
        name = extract_symbol_name(param_node)
        name ? [name] : []
      else
        []
      end
    end

    def self.compile_cond(branches, ctx, tail:)
      end_jumps = []
      branches.each do |branch|
        branch = unwrap(branch)
        next unless rule_name(branch) == "list"

        body_items = children(branch).select { |c| rule_name(c) == "list_body" }
        next if body_items.empty?

        clause_children = children(body_items.first).select { |c| rule_name(c) == "sexpr" }
        next unless clause_children.size >= 2

        predicate = clause_children[0]
        consequent = clause_children[1]

        compile_sexpr(predicate, ctx, tail: false)
        jump_idx = ctx.emit(LOP::JUMP_IF_FALSE, 0)

        compile_sexpr(consequent, ctx, tail: tail)
        end_jumps << ctx.emit(LOP::JUMP, 0)

        ctx.patch_jump(jump_idx, ctx.instructions.size)
      end
      ctx.emit(LOP::LOAD_NIL)
      target = ctx.instructions.size
      end_jumps.each { |j| ctx.patch_jump(j, target) }
    end

    def self.compile_quote_form(args, ctx)
      compile_quoted_value(args[0], ctx)
    end

    def self.compile_quoted(node, ctx)
      cs = children(node)
      # quoted = QUOTE sexpr
      sexpr = cs.find { |c| rule_name(c) == "sexpr" }
      compile_quoted_value(sexpr, ctx)
    end

    def self.compile_quoted_value(node, ctx)
      return ctx.emit(LOP::LOAD_NIL) unless node
      node = unwrap(node)
      case rule_name(node)
      when "atom"
        tok = children(node).first
        case tok&.type&.to_s
        when "NUMBER"
          val = tok.value.include?(".") ? tok.value.to_f : tok.value.to_i
          ctx.emit(LOP::LOAD_CONST, ctx.add_constant(val))
        when "STRING"
          ctx.emit(LOP::LOAD_CONST, ctx.add_constant(tok.value[1..-2]))
        when "SYMBOL"
          ctx.emit(LOP::MAKE_SYMBOL, ctx.add_constant(tok.value))
        else
          ctx.emit(LOP::LOAD_NIL)
        end
      when "list"
        # Build a cons chain from right to left
        body_items = children(node).select { |c| rule_name(c) == "list_body" }
        if body_items.empty?
          ctx.emit(LOP::LOAD_NIL)
        else
          sexprs = children(body_items.first).select { |c| rule_name(c) == "sexpr" }
          if sexprs.empty?
            ctx.emit(LOP::LOAD_NIL)
          else
            ctx.emit(LOP::LOAD_NIL)
            sexprs.reverse_each do |s|
              compile_quoted_value(s, ctx)
              ctx.emit(LOP::CONS)
            end
          end
        end
      else
        ctx.emit(LOP::LOAD_NIL)
      end
    end

    # ================================================================
    # Builtins and Calls
    # ================================================================

    def self.compile_builtin_1(args, ctx, opcode)
      compile_sexpr(args[0], ctx, tail: false) if args[0]
      ctx.emit(opcode)
    end

    def self.compile_builtin_2(args, ctx, opcode)
      compile_sexpr(args[0], ctx, tail: false) if args[0]
      compile_sexpr(args[1], ctx, tail: false) if args[1]
      ctx.emit(opcode)
    end

    def self.compile_arith(args, ctx, opcode)
      return ctx.emit(LOP::LOAD_CONST, ctx.add_constant(0)) if args.empty?
      compile_sexpr(args[0], ctx, tail: false)
      args[1..].each do |arg|
        compile_sexpr(arg, ctx, tail: false)
        ctx.emit(opcode)
      end
    end

    def self.compile_print(args, ctx)
      compile_sexpr(args[0], ctx, tail: false) if args[0]
      ctx.emit(LOP::PRINT)
    end

    def self.compile_call(fn_node, args, ctx, tail:)
      args.each { |arg| compile_sexpr(arg, ctx, tail: false) }
      compile_sexpr(fn_node, ctx, tail: false)
      opcode = tail ? LOP::TAIL_CALL : LOP::CALL_FUNCTION
      ctx.emit(opcode, args.size)
    end
  end
end
