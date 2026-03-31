# CodingAdventures::StarlarkAstToBytecodeCompiler

A Pure Perl compiler that translates Starlark ASTs to stack-based bytecode.

## Usage

```perl
use CodingAdventures::StarlarkAstToBytecodeCompiler;

my $C = 'CodingAdventures::StarlarkAstToBytecodeCompiler';

my $tree = $C->ast_node('file', [
    $C->ast_node('statement', [
        $C->ast_node('simple_stmt', [
            $C->ast_node('assign_stmt', [
                $C->ast_node('identifier', [ $C->token_node('NAME', 'x') ]),
                $C->token_node('OP', '='),
                $C->ast_node('atom', [ $C->token_node('INT', '42') ]),
            ])
        ])
    ])
]);

my $co = $C->compile_ast($tree);
# $co->{instructions}[0]{opcode} == 0x01  (OP_LOAD_CONST)
# $co->{constants}[0] == 42
# $co->{names}[0] eq 'x'
```

## Installation

```sh
cpanm --installdeps .
```

## Testing

```sh
prove -l -v t/
```
