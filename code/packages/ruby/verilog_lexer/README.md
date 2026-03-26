# Verilog Lexer

A Ruby gem that tokenizes Verilog HDL source code using the grammar-driven lexer engine. Includes a preprocessor for resolving compiler directives (`define, `ifdef, `include, `timescale).

## Overview

This gem is a wrapper around `coding_adventures_lexer`'s `GrammarLexer`. It loads the `verilog.tokens` grammar file and feeds it to the general-purpose lexer engine to tokenize Verilog hardware description language source code.

Verilog has unique features not found in software languages:
- Sized numbers with bit-width: `8'hFF`, `4'b1010`
- System tasks: `$display`, `$time`
- Compiler directives: `` `define ``, `` `ifdef ``
- Escaped identifiers: `\bus[0]`

The optional preprocessor resolves directives before tokenization.

## Usage

```ruby
require "coding_adventures_verilog_lexer"

# Basic tokenization
tokens = CodingAdventures::VerilogLexer.tokenize("wire [7:0] data;")
tokens.each { |t| puts t }

# With preprocessor enabled
source = <<~VERILOG
  `define WIDTH 8
  wire [`WIDTH-1:0] data;
VERILOG
tokens = CodingAdventures::VerilogLexer.tokenize(source, preprocess: true)

# Preprocessor standalone
processed = CodingAdventures::VerilogLexer::Preprocessor.process(source)
```

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
