use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::TypescriptLexer; 1 }, 'module loads' );

# ============================================================================
# Helpers
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::TypescriptLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::TypescriptLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1, '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# JavaScript keywords (all inherited by TypeScript)
# ============================================================================

subtest 'keyword: var' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('var');
    is( $tokens->[0]{type}, 'VAR', 'type is VAR' );
};

subtest 'keyword: let' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('let');
    is( $tokens->[0]{type}, 'LET', 'type is LET' );
};

subtest 'keyword: const' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('const');
    is( $tokens->[0]{type}, 'CONST', 'type is CONST' );
};

subtest 'keyword: function' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('function');
    is( $tokens->[0]{type}, 'FUNCTION', 'type is FUNCTION' );
};

subtest 'keyword: return' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('return');
    is( $tokens->[0]{type}, 'RETURN', 'type is RETURN' );
};

subtest 'keywords: if and else' => sub {
    is( types_of('if else'), [qw(IF ELSE)], 'if and else' );
};

subtest 'keywords: for and while' => sub {
    is( types_of('for while'), [qw(FOR WHILE)], 'for and while' );
};

subtest 'keyword: class' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('class');
    is( $tokens->[0]{type}, 'CLASS', 'type is CLASS' );
};

subtest 'keywords: true false null undefined' => sub {
    is(
        types_of('true false null undefined'),
        [qw(TRUE FALSE NULL UNDEFINED)],
        'JS boolean/null literals'
    );
};

subtest 'keywords: typeof and instanceof' => sub {
    is( types_of('typeof instanceof'), [qw(TYPEOF INSTANCEOF)], 'typeof instanceof' );
};

# ============================================================================
# TypeScript-specific keywords
# ============================================================================

subtest 'keyword: interface' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('interface');
    is( $tokens->[0]{type},  'INTERFACE', 'type is INTERFACE' );
    is( $tokens->[0]{value}, 'interface', 'value is interface' );
};

subtest 'keyword: type' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('type');
    is( $tokens->[0]{type},  'TYPE', 'type is TYPE' );
    is( $tokens->[0]{value}, 'type', 'value is type' );
};

subtest 'keyword: enum' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('enum');
    is( $tokens->[0]{type},  'ENUM', 'type is ENUM' );
    is( $tokens->[0]{value}, 'enum', 'value is enum' );
};

subtest 'keyword: namespace' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('namespace');
    is( $tokens->[0]{type},  'NAMESPACE', 'type is NAMESPACE' );
    is( $tokens->[0]{value}, 'namespace', 'value is namespace' );
};

subtest 'keyword: declare' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('declare');
    is( $tokens->[0]{type},  'DECLARE', 'type is DECLARE' );
    is( $tokens->[0]{value}, 'declare', 'value is declare' );
};

subtest 'keyword: readonly' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('readonly');
    is( $tokens->[0]{type},  'READONLY', 'type is READONLY' );
    is( $tokens->[0]{value}, 'readonly', 'value is readonly' );
};

subtest 'keyword: abstract' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('abstract');
    is( $tokens->[0]{type},  'ABSTRACT', 'type is ABSTRACT' );
    is( $tokens->[0]{value}, 'abstract', 'value is abstract' );
};

subtest 'keyword: implements' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('implements');
    is( $tokens->[0]{type},  'IMPLEMENTS', 'type is IMPLEMENTS' );
    is( $tokens->[0]{value}, 'implements', 'value is implements' );
};

subtest 'keyword: extends' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('extends');
    is( $tokens->[0]{type},  'EXTENDS', 'type is EXTENDS' );
    is( $tokens->[0]{value}, 'extends', 'value is extends' );
};

subtest 'keyword: keyof' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('keyof');
    is( $tokens->[0]{type},  'KEYOF', 'type is KEYOF' );
    is( $tokens->[0]{value}, 'keyof', 'value is keyof' );
};

subtest 'keyword: infer' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('infer');
    is( $tokens->[0]{type},  'INFER', 'type is INFER' );
    is( $tokens->[0]{value}, 'infer', 'value is infer' );
};

subtest 'keyword: never' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('never');
    is( $tokens->[0]{type},  'NEVER', 'type is NEVER' );
    is( $tokens->[0]{value}, 'never', 'value is never' );
};

subtest 'keyword: unknown' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('unknown');
    is( $tokens->[0]{type},  'UNKNOWN', 'type is UNKNOWN' );
    is( $tokens->[0]{value}, 'unknown', 'value is unknown' );
};

subtest 'keyword: any' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('any');
    is( $tokens->[0]{type},  'ANY', 'type is ANY' );
    is( $tokens->[0]{value}, 'any', 'value is any' );
};

subtest 'keyword: void' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('void');
    is( $tokens->[0]{type},  'VOID', 'type is VOID' );
    is( $tokens->[0]{value}, 'void', 'value is void' );
};

subtest 'keyword: boolean' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('boolean');
    is( $tokens->[0]{type},  'BOOLEAN', 'type is BOOLEAN' );
    is( $tokens->[0]{value}, 'boolean', 'value is boolean' );
};

subtest 'keyword: object' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('object');
    is( $tokens->[0]{type},  'OBJECT', 'type is OBJECT' );
    is( $tokens->[0]{value}, 'object', 'value is object' );
};

subtest 'keyword: symbol' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('symbol');
    is( $tokens->[0]{type},  'SYMBOL', 'type is SYMBOL' );
    is( $tokens->[0]{value}, 'symbol', 'value is symbol' );
};

subtest 'keyword: bigint' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('bigint');
    is( $tokens->[0]{type},  'BIGINT', 'type is BIGINT' );
    is( $tokens->[0]{value}, 'bigint', 'value is bigint' );
};

# ============================================================================
# Access modifiers
# ============================================================================

subtest 'access modifier: public' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('public');
    is( $tokens->[0]{type},  'PUBLIC', 'type is PUBLIC' );
    is( $tokens->[0]{value}, 'public', 'value is public' );
};

subtest 'access modifier: private' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('private');
    is( $tokens->[0]{type},  'PRIVATE', 'type is PRIVATE' );
    is( $tokens->[0]{value}, 'private', 'value is private' );
};

subtest 'access modifier: protected' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('protected');
    is( $tokens->[0]{type},  'PROTECTED', 'type is PROTECTED' );
    is( $tokens->[0]{value}, 'protected', 'value is protected' );
};

# ============================================================================
# Identifiers and basic literals
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('myVar');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, 'myVar', 'value is myVar' );
};

subtest 'integer number literal' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'double-quoted string literal' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

# ============================================================================
# Operators
# ============================================================================

subtest 'strict equals ===' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('===');
    is( $tokens->[0]{type}, 'STRICT_EQUALS', 'type is STRICT_EQUALS' );
};

subtest 'strict not equals !==' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('!==');
    is( $tokens->[0]{type}, 'STRICT_NOT_EQUALS', 'type is STRICT_NOT_EQUALS' );
};

subtest 'arrow =>' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('=>');
    is( $tokens->[0]{type}, 'ARROW', 'type is ARROW' );
};

subtest 'less than < (for generics)' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('<');
    is( $tokens->[0]{type}, 'LESS_THAN', 'type is LESS_THAN' );
};

subtest 'greater than > (for generics)' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('>');
    is( $tokens->[0]{type}, 'GREATER_THAN', 'type is GREATER_THAN' );
};

# ============================================================================
# TypeScript constructs
# ============================================================================

subtest 'type annotation: x: number' => sub {
    # x : number  →  NAME COLON NUMBER
    is(
        types_of('x: number'),
        [qw(NAME COLON NUMBER)],
        'type annotation types'
    );
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('x: number');
    is( $tokens->[0]{value}, 'x',      'identifier value' );
    is( $tokens->[2]{value}, 'number', 'type keyword value' );
};

subtest 'type annotation: x: string' => sub {
    is( types_of('x: string'), [qw(NAME COLON STRING)], 'string type annotation' );
};

subtest 'generic type: Array<string>' => sub {
    # Array < string >  →  NAME LESS_THAN STRING GREATER_THAN
    is(
        types_of('Array<string>'),
        [qw(NAME LESS_THAN STRING GREATER_THAN)],
        'generic type types'
    );
};

subtest 'interface declaration: interface Foo { }' => sub {
    is(
        types_of('interface Foo { }'),
        [qw(INTERFACE NAME LBRACE RBRACE)],
        'interface declaration types'
    );
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('interface Foo { }');
    is( $tokens->[1]{value}, 'Foo', 'interface name is Foo' );
};

subtest 'interface with body: interface Point { x: number; y: number; }' => sub {
    is(
        types_of('interface Point { x: number; y: number; }'),
        [qw(INTERFACE NAME LBRACE
            NAME COLON NUMBER SEMICOLON
            NAME COLON NUMBER SEMICOLON
            RBRACE)],
        'interface body types'
    );
};

subtest 'enum declaration: enum Color { Red, Green, Blue }' => sub {
    is(
        types_of('enum Color { Red, Green, Blue }'),
        [qw(ENUM NAME LBRACE
            NAME COMMA NAME COMMA NAME
            RBRACE)],
        'enum declaration types'
    );
};

subtest 'access modifier in class: public getName(): string { }' => sub {
    # public getName ( ) : string { }
    is(
        types_of('public getName(): string { }'),
        [qw(PUBLIC NAME LPAREN RPAREN COLON STRING LBRACE RBRACE)],
        'access modifier in method types'
    );
};

subtest 'generic constraint: <T extends K>' => sub {
    is(
        types_of('<T extends K>'),
        [qw(LESS_THAN NAME EXTENDS NAME GREATER_THAN)],
        'generic extends constraint types'
    );
};

subtest 'declare statement: declare module foo { }' => sub {
    is(
        types_of('declare module foo { }'),
        [qw(DECLARE NAME NAME LBRACE RBRACE)],
        'declare module types'
    );
};

subtest 'readonly property: readonly id: number' => sub {
    is(
        types_of('readonly id: number'),
        [qw(READONLY NAME COLON NUMBER)],
        'readonly property types'
    );
};

subtest 'abstract class: abstract class Shape { }' => sub {
    is(
        types_of('abstract class Shape { }'),
        [qw(ABSTRACT CLASS NAME LBRACE RBRACE)],
        'abstract class types'
    );
};

subtest 'implements clause: class Dog implements Animal { }' => sub {
    is(
        types_of('class Dog implements Animal { }'),
        [qw(CLASS NAME IMPLEMENTS NAME LBRACE RBRACE)],
        'implements clause types'
    );
};

subtest 'extends clause: class Cat extends Animal { }' => sub {
    is(
        types_of('class Cat extends Animal { }'),
        [qw(CLASS NAME EXTENDS NAME LBRACE RBRACE)],
        'extends clause types'
    );
};

subtest 'keyof type operator: keyof T' => sub {
    is( types_of('keyof T'), [qw(KEYOF NAME)], 'keyof operator types' );
};

subtest 'as type assertion: x as string' => sub {
    # x → NAME, as → AS (shared with JS), string → STRING keyword
    is( types_of('x as string'), [qw(NAME AS STRING)], 'type assertion with as' );
};

subtest 'const declaration with type: const x: number = 1;' => sub {
    is(
        types_of('const x: number = 1;'),
        [qw(CONST NAME COLON NUMBER EQUALS NUMBER SEMICOLON)],
        'const with type annotation'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('interface Foo { }'),
        [qw(INTERFACE NAME LBRACE RBRACE)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs and newlines consumed silently' => sub {
    is(
        types_of("interface\nFoo\n{\n}"),
        [qw(INTERFACE NAME LBRACE RBRACE)],
        'newlines consumed'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: interface Foo' => sub {
    # i n t e r f a c e   F o o
    # 1 . . . . . . . . 10 11
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('interface Foo');
    is( $tokens->[0]{col},  1, 'interface at col 1' );
    is( $tokens->[1]{col}, 11, 'Foo at col 11' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('interface Foo { }');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('interface Foo { }');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character @ raises die' => sub {
    ok(
        dies { CodingAdventures::TypescriptLexer->tokenize('@') },
        'unexpected @ causes die'
    );
};

subtest 'backtick raises die (template literals not in grammar)' => sub {
    ok(
        dies { CodingAdventures::TypescriptLexer->tokenize('`hello`') },
        'backtick causes die'
    );
};

done_testing;
