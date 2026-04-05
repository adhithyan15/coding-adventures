use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::EcmascriptES3Lexer; 1 }, 'module loads' );

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# Empty
subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('');
    is( scalar @$tokens, 1, '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ES1 keywords
subtest 'keyword: var' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('var');
    is( $tokens->[0]{type}, 'VAR', 'type is VAR' );
};

subtest 'keyword: function' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('function');
    is( $tokens->[0]{type}, 'FUNCTION', 'type is FUNCTION' );
};

subtest 'keywords: true false null' => sub {
    is( types_of('true false null'), [qw(TRUE FALSE NULL)], 'literal keywords' );
};

# New ES3 keywords
subtest 'keyword: try' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('try');
    is( $tokens->[0]{type}, 'TRY', 'type is TRY' );
};

subtest 'keyword: catch' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('catch');
    is( $tokens->[0]{type}, 'CATCH', 'type is CATCH' );
};

subtest 'keyword: finally' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('finally');
    is( $tokens->[0]{type}, 'FINALLY', 'type is FINALLY' );
};

subtest 'keyword: throw' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('throw');
    is( $tokens->[0]{type}, 'THROW', 'type is THROW' );
};

subtest 'keyword: instanceof' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('instanceof');
    is( $tokens->[0]{type}, 'INSTANCEOF', 'type is INSTANCEOF' );
};

# Strict equality (new in ES3)
subtest 'strict equals ===' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('===');
    is( $tokens->[0]{type}, 'STRICT_EQUALS', 'type is STRICT_EQUALS' );
};

subtest 'strict not equals !==' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('!==');
    is( $tokens->[0]{type}, 'STRICT_NOT_EQUALS', 'type is STRICT_NOT_EQUALS' );
};

subtest 'loose equals == still works' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('==');
    is( $tokens->[0]{type}, 'EQUALS_EQUALS', 'type is EQUALS_EQUALS' );
};

# Identifiers and literals
subtest 'identifier' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('myVar');
    is( $tokens->[0]{type}, 'NAME', 'type is NAME' );
};

subtest 'number' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('42');
    is( $tokens->[0]{type}, 'NUMBER', 'type is NUMBER' );
};

subtest 'string' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('"hello"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
};

# Operators
subtest 'unsigned right shift >>>' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('>>>');
    is( $tokens->[0]{type}, 'UNSIGNED_RIGHT_SHIFT', 'type is UNSIGNED_RIGHT_SHIFT' );
};

# Composite
subtest 'var declaration' => sub {
    is( types_of('var x = 1;'), [qw(VAR NAME EQUALS NUMBER SEMICOLON)], 'var decl' );
};

subtest 'try/catch expression' => sub {
    my $t = types_of('try { } catch (e) { }');
    is( $t->[0], 'TRY', 'starts with TRY' );
};

subtest 'instanceof expression' => sub {
    is( types_of('x instanceof Foo'), [qw(NAME INSTANCEOF NAME)], 'instanceof' );
};

# Position
subtest 'column tracking' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('var x = 1;');
    is( $tokens->[0]{col}, 1, 'var at col 1' );
};

# EOF and errors
subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('1');
    is( $tokens->[-1]{type}, 'EOF', 'last token is EOF' );
};

subtest 'unexpected character # raises die' => sub {
    ok( dies { CodingAdventures::EcmascriptES3Lexer->tokenize('#') }, '# causes die' );
};

done_testing;
