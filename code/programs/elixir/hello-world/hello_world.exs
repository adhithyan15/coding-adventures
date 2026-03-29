# The first program — Hello, World!
#
# This is the starting point for the entire coding-adventures project.
# The long-term goal is to trace this simple program all the way down
# through the computing stack:
#
#   Source code (this file)
#   → Lexer (tokenize)
#   → Parser (build AST)
#   → Compiler (emit bytecode or ARM assembly)
#   → Virtual Machine (execute bytecode)
#   → ARM Simulator (execute machine instructions)
#   → CPU Simulator (fetch-decode-execute cycle)
#   → ALU (arithmetic operations)
#   → Logic Gates (AND, OR, NOT — the foundation)

IO.puts("Hello, World!")
