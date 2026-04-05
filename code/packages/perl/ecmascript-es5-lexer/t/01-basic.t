use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::EcmascriptES5Lexer; 1 }, 'module loads' );

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# Empty
subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('');
    is( scalar @$tokens, 1, '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ES5-specific: debugger keyword
subtest 'keyword: debugger' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('debugger');
    is( $tokens->[0]{type}, 'DEBUGGER', 'type is DEBUGGER' );
    is( $tokens->[0]{value}, 'debugger', 'value is debugger' );
};

subtest 'debugger statement' => sub {
    is( types_of('debugger;'), [qw(DEBUGGER SEMICOLON)], 'debugger statement' );
};

# ES3 keywords
subtest 'keyword: var' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('var');
    is( $tokens->[0]{type}, 'VAR', 'type is VAR' );
};

subtest 'keyword: try' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('try');
    is( $tokens->[0]{type}, 'TRY', 'type is TRY' );
};

subtest 'keyword: catch' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('catch');
    is( $tokens->[0]{type}, 'CATCH', 'type is CATCH' );
};

subtest 'keyword: instanceof' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('instanceof');
    is( $tokens->[0]{type}, 'INSTANCEOF', 'type is INSTANCEOF' );
};

subtest 'keywords: true false null' => sub {
    is( types_of('true false null'), [qw(TRUE FALSE NULL)], 'literal keywords' );
};

# Strict equality
subtest 'strict equals ===' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('===');
    is( $tokens->[0]{type}, 'STRICT_EQUALS', 'type is STRICT_EQUALS' );
};

subtest 'strict not equals !==' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('!==');
    is( $tokens->[0]{type}, 'STRICT_NOT_EQUALS', 'type is STRICT_NOT_EQUALS' );
};

# Identifiers and literals
subtest 'identifier' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('myVar');
    is( $tokens->[0]{type}, 'NAME', 'type is NAME' );
};

subtest 'number' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('42');
    is( $tokens->[0]{type}, 'NUMBER', 'type is NUMBER' );
};

subtest 'string' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('"hello"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
};

# Composite
subtest 'var declaration' => sub {
    is( types_of('var x = 1;'), [qw(VAR NAME EQUALS NUMBER SEMICOLON)], 'var decl' );
};

subtest 'strict equality: a === b' => sub {
    is( types_of('a === b'), [qw(NAME STRICT_EQUALS NAME)], 'strict equality' );
};

# Position
subtest 'column tracking' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('var x = 1;');
    is( $tokens->[0]{col}, 1, 'var at col 1' );
};

# EOF and errors
subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('1');
    is( $tokens->[-1]{type}, 'EOF', 'last token is EOF' );
};

subtest 'unexpected character # raises die' => sub {
    ok( dies { CodingAdventures::EcmascriptES5Lexer->tokenize('#') }, '# causes die' );
};

done_testing;
