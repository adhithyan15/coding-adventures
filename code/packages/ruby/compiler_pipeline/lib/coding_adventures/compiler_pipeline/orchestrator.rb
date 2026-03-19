# frozen_string_literal: true

# === Pipeline Orchestrator -- Wiring the Full Computing Stack ===
#
# Imagine a factory assembly line. Raw steel enters at one end and a finished
# car rolls out at the other. Between those two points, a dozen stations each
# do one specific job: stamping, welding, painting, assembly. No single station
# builds the whole car -- each one transforms its input and passes the result
# downstream.
#
# A **compiler pipeline** works the same way. Raw source code enters at one
# end, and executable results come out the other. Our pipeline has four
# stations:
#
#     Source code  ->  Lexer  ->  Parser  ->  Compiler  ->  VM
#
# 1. **Lexer** (tokenizer): Reads raw characters and groups them into
#    meaningful tokens -- identifiers, numbers, operators, keywords.
#
# 2. **Parser**: Takes the flat token stream and builds a tree structure
#    (the Abstract Syntax Tree) that encodes precedence and grouping.
#
# 3. **Compiler**: Walks the AST and emits flat bytecode instructions for
#    a stack machine. Tree structure becomes linear execution.
#
# 4. **Virtual Machine**: Executes the bytecode instructions one by one,
#    maintaining a stack, variables, and captured output.
#
# === Why Capture Traces? ===
#
# The pipeline does not just *run* code -- it **records** what happened at
# every stage. This is critical for the HTML visualizer, which lets a learner
# see exactly how "x = 1 + 2" transforms at each step:
#
# - The lexer stage shows which characters became which tokens.
# - The parser stage shows the tree structure.
# - The compiler stage shows the flat bytecode instructions.
# - The VM stage shows the stack evolving as each instruction executes.

module CodingAdventures
  module CompilerPipeline
    # Captured output from the lexer stage.
    LexerStage = Data.define(:tokens, :token_count, :source)

    # Captured output from the parser stage.
    ParserStage = Data.define(:ast, :ast_dict)

    # Captured output from the compiler stage.
    CompilerStage = Data.define(:code, :instructions_text, :constants, :names)

    # Captured output from the VM execution stage.
    VMStage = Data.define(:traces, :final_variables, :output)

    # The complete result of running source code through all stages.
    PipelineResult = Data.define(:source, :lexer_stage, :parser_stage,
      :compiler_stage, :vm_stage)

    # ------------------------------------------------------------------
    # AST-to-dictionary conversion
    # ------------------------------------------------------------------
    #
    # The HTML visualizer needs a JSON-serializable representation of the
    # AST so it can render the tree as an interactive diagram.

    # Convert an AST node to a JSON-serializable hash.
    #
    # Walks the AST recursively, converting each node into a plain hash
    # with a "type" key and type-specific fields.
    def self.ast_to_dict(node)
      case node
      when CodingAdventures::Parser::Program
        {
          "type" => "Program",
          "statements" => node.statements.map { |s| ast_to_dict(s) }
        }

      when CodingAdventures::Parser::Assignment
        {
          "type" => "Assignment",
          "target" => ast_to_dict(node.target),
          "value" => ast_to_dict(node.value)
        }

      when CodingAdventures::Parser::BinaryOp
        {
          "type" => "BinaryOp",
          "op" => node.op,
          "left" => ast_to_dict(node.left),
          "right" => ast_to_dict(node.right)
        }

      when CodingAdventures::Parser::NumberLiteral
        {"type" => "NumberLiteral", "value" => node.value}

      when CodingAdventures::Parser::StringLiteral
        {"type" => "StringLiteral", "value" => node.value}

      when CodingAdventures::Parser::Name
        {"type" => "Name", "name" => node.name}

      else
        {"type" => node.class.name, "repr" => node.inspect}
      end
    end

    # ------------------------------------------------------------------
    # Instruction-to-text conversion
    # ------------------------------------------------------------------
    #
    # The visualizer shows human-readable bytecode like:
    #   LOAD_CONST 0 (42)
    #   STORE_NAME 0 ('x')
    #   ADD

    # Convert a bytecode instruction to human-readable text.
    def self.instruction_to_text(instr, code)
      opcode_name = instr.opcode.to_s

      if !instr.operand.nil?
        if instr.opcode == :LOAD_CONST && instr.operand.is_a?(Integer)
          if instr.operand >= 0 && instr.operand < code.constants.length
            return "#{opcode_name} #{instr.operand} (#{code.constants[instr.operand].inspect})"
          end
        elsif [:STORE_NAME, :LOAD_NAME].include?(instr.opcode) && instr.operand.is_a?(Integer)
          if instr.operand >= 0 && instr.operand < code.names.length
            return "#{opcode_name} #{instr.operand} (#{code.names[instr.operand].inspect})"
          end
        end

        return "#{opcode_name} #{instr.operand}"
      end

      opcode_name
    end

    # ------------------------------------------------------------------
    # The Pipeline class -- the assembly line itself
    # ------------------------------------------------------------------

    # The main pipeline orchestrator.
    #
    # Chains: Source -> Lexer -> Parser -> Compiler -> VM
    #
    # This class coordinates the specialized packages. It does not do any
    # of the actual work (tokenizing, parsing, compiling, executing) --
    # that is handled by the dedicated gems. Instead, it wires them
    # together and captures the output at every stage.
    class Orchestrator
      # Run source code through the full pipeline.
      #
      # Given source code like "x = 1 + 2", runs all four stages and
      # returns a PipelineResult containing captured data from every stage.
      def run(source, keywords: nil)
        # ---------------------------------------------------------------
        # Stage 1: Lexing -- characters to tokens
        # ---------------------------------------------------------------
        kw = keywords || []
        lexer = CodingAdventures::Lexer::Tokenizer.new(source, keywords: kw)
        tokens = lexer.tokenize

        lexer_stage = LexerStage.new(
          tokens: tokens,
          token_count: tokens.length,
          source: source
        )

        # ---------------------------------------------------------------
        # Stage 2: Parsing -- tokens to AST
        # ---------------------------------------------------------------
        parser = CodingAdventures::Parser::RecursiveDescentParser.new(tokens)
        ast = parser.parse

        parser_stage = ParserStage.new(
          ast: ast,
          ast_dict: CompilerPipeline.ast_to_dict(ast)
        )

        # ---------------------------------------------------------------
        # Stage 3: Compilation -- AST to bytecode
        # ---------------------------------------------------------------
        compiler = CodingAdventures::BytecodeCompiler::Compiler.new
        code = compiler.compile(ast)

        compiler_stage = CompilerStage.new(
          code: code,
          instructions_text: code.instructions.map { |instr|
            CompilerPipeline.instruction_to_text(instr, code)
          },
          constants: code.constants.dup,
          names: code.names.dup
        )

        # ---------------------------------------------------------------
        # Stage 4: VM Execution -- bytecode to results
        # ---------------------------------------------------------------
        vm = CodingAdventures::VirtualMachine::VM.new
        traces = vm.execute(code)

        vm_stage = VMStage.new(
          traces: traces,
          final_variables: vm.variables.dup,
          output: vm.output.dup
        )

        # ---------------------------------------------------------------
        # Bundle everything into a PipelineResult
        # ---------------------------------------------------------------
        PipelineResult.new(
          source: source,
          lexer_stage: lexer_stage,
          parser_stage: parser_stage,
          compiler_stage: compiler_stage,
          vm_stage: vm_stage
        )
      end
    end
  end
end
