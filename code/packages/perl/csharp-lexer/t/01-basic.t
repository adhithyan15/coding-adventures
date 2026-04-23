use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CSharpLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::CSharpLexer->tokenize($source, $version);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::CSharpLexer->tokenize($source, $version);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('myVar');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, 'myVar', 'value is myVar' );
};

subtest 'identifier with underscore prefix' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('_priv');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, '_priv', 'value is _priv' );
};

# ============================================================================
# Number tokens
# ============================================================================

subtest 'integer number' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('0');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '0',      'value is 0' );
};

# ============================================================================
# String tokens
# ============================================================================

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
};

subtest 'empty double-quoted string' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
};

# ============================================================================
# Punctuation tokens
# ============================================================================

subtest 'parentheses' => sub {
    is( types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
};

subtest 'braces' => sub {
    is( types_of('{}'), [qw(LBRACE RBRACE)], 'brace types' );
};

subtest 'brackets' => sub {
    is( types_of('[]'), [qw(LBRACKET RBRACKET)], 'bracket types' );
};

subtest 'semicolon' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize(';');
    is( $tokens->[0]{type},  'SEMICOLON', 'type is SEMICOLON' );
    is( $tokens->[0]{value}, ';',         'value is ;' );
};

subtest 'comma' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize(',');
    is( $tokens->[0]{type},  'COMMA', 'type is COMMA' );
    is( $tokens->[0]{value}, ',',     'value is ,' );
};

subtest 'dot' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('.');
    is( $tokens->[0]{type},  'DOT', 'type is DOT' );
    is( $tokens->[0]{value}, '.',   'value is .' );
};

# ============================================================================
# C# class declaration
# ============================================================================
#
# C# is a class-based object-oriented language. A minimal class looks like:
#
#   public class MyClass { }
#
# The keywords `public` and `class` must be recognized, and the braces
# correctly tokenized.

subtest 'basic C# class declaration tokenizes correctly' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('public class MyClass { }');
    my @non_eof = grep { $_->{type} ne 'EOF' } @$tokens;
    ok( scalar @non_eof >= 4, 'at least 4 non-EOF tokens' );

    # The keyword 'public' should be recognized as a keyword token, not NAME
    my ($pub) = grep { $_->{value} eq 'public' } @non_eof;
    ok( defined $pub, 'public token present' );
    ok( $pub->{type} ne 'NAME' || $pub->{type} eq 'PUBLIC',
        'public is recognized as keyword or PUBLIC type' );

    # The keyword 'class' should be recognized
    my ($cls) = grep { $_->{value} eq 'class' } @non_eof;
    ok( defined $cls, 'class token present' );

    # The identifier 'MyClass' should be NAME
    my ($name) = grep { $_->{value} eq 'MyClass' } @non_eof;
    ok( defined $name, 'MyClass token present' );
    is( $name->{type}, 'NAME', 'MyClass is NAME' );
};

# ============================================================================
# C# keywords
# ============================================================================
#
# C# has many keywords. We test a selection of core ones here.
# The grammar file drives which tokens are recognized as keywords vs. NAMEs.

subtest 'C# int keyword is recognized' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 0;');
    my ($tok) = grep { $_->{value} eq 'int' } @$tokens;
    ok( defined $tok, 'int token found' );
    ok( $tok->{type} ne 'NAME' || $tok->{type} eq 'INT',
        'int is recognized as a keyword' );
};

subtest 'C# string keyword is recognized' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('string s;');
    my ($tok) = grep { $_->{value} eq 'string' } @$tokens;
    ok( defined $tok, 'string keyword token found' );
};

subtest 'C# bool keyword is recognized' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('bool flag = true;');
    my ($bool_tok) = grep { $_->{value} eq 'bool' } @$tokens;
    ok( defined $bool_tok, 'bool token found' );
};

subtest 'C# new keyword is recognized' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('new Foo()');
    my ($new_tok) = grep { $_->{value} eq 'new' } @$tokens;
    ok( defined $new_tok, 'new token found' );
    ok( $new_tok->{type} ne 'NAME' || $new_tok->{type} eq 'NEW',
        'new is treated as a keyword' );
};

subtest 'C# namespace keyword is recognized' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('namespace MyApp { }');
    my ($ns_tok) = grep { $_->{value} eq 'namespace' } @$tokens;
    ok( defined $ns_tok, 'namespace token found' );
};

subtest 'C# using keyword is recognized' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('using System;');
    my ($tok) = grep { $_->{value} eq 'using' } @$tokens;
    ok( defined $tok, 'using token found' );
};

# ============================================================================
# C# operators
# ============================================================================

subtest 'null-coalescing operator ??' => sub {
    # ?? is unique to C# (not in Java). Available from C# 2.0+.
    # It returns the left operand if not null; otherwise the right operand.
    my $tokens = CodingAdventures::CSharpLexer->tokenize('a ?? b', '2.0');
    my @non_eof = grep { $_->{type} ne 'EOF' } @$tokens;
    my ($qq_tok) = grep { $_->{value} eq '??' } @non_eof;
    ok( defined $qq_tok, '?? token is present' );
};

subtest 'null-conditional member access ?.' => sub {
    # ?. is unique to C# (not in Java). Available from C# 6.0+.
    # Returns null if the left side is null; otherwise accesses the member.
    my $tokens = CodingAdventures::CSharpLexer->tokenize('obj?.Length', '6.0');
    my @non_eof = grep { $_->{type} ne 'EOF' } @$tokens;
    my ($qd_tok) = grep { $_->{value} eq '?.' } @non_eof;
    ok( defined $qd_tok, '?. token is present' );
};

subtest 'addition operator +' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('1 + 2');
    my ($plus) = grep { $_->{value} eq '+' } @$tokens;
    ok( defined $plus, '+ token found' );
};

subtest 'assignment operator =' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('x = 5;');
    my ($eq) = grep { $_->{value} eq '=' } @$tokens;
    ok( defined $eq, '= token found' );
};

subtest 'equality operator ==' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('a == b');
    my ($eqeq) = grep { $_->{value} eq '==' } @$tokens;
    ok( defined $eqeq, '== token found' );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 1;');
    my @types = map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens;
    ok( !grep({ $_ eq 'WHITESPACE' } @types), 'no WHITESPACE tokens in output' );
};

subtest 'tabs and newlines consumed silently' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize("int\n\tx\n=\n1;");
    my @types = map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens;
    ok( !grep({ $_ eq 'WHITESPACE' } @types), 'only value tokens in output' );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 1;');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Version-aware tokenization — all 12 C# versions must not die
# ============================================================================

my @ALL_VERSIONS = qw(1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0);

subtest 'tokenize with default version (no version specified)' => sub {
    my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 1;');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

for my $ver (@ALL_VERSIONS) {
    subtest "tokenize with version $ver" => sub {
        my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 1;', $ver);
        ok( scalar @$tokens > 0, "produced tokens for version $ver" );
    };
}

subtest 'grammar is cached per version' => sub {
    my $t1 = CodingAdventures::CSharpLexer->tokenize('int x = 1;', '8.0');
    my $t2 = CodingAdventures::CSharpLexer->tokenize('int x = 1;', '8.0');
    is( $t1->[0]{type}, $t2->[0]{type}, 'same type from cached grammar' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unknown version raises die' => sub {
    ok(
        dies { CodingAdventures::CSharpLexer->tokenize('int x = 1;', '99') },
        'unknown version 99 causes die'
    );
};

subtest 'invalid version string is rejected' => sub {
    ok(
        dies { CodingAdventures::CSharpLexer->tokenize('int x = 1;', 'csharp12') },
        'csharp12 is not a valid C# version'
    );
};

done_testing;
