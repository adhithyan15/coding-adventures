use Test2::V0;

# ============================================================================
# t/02-lexer.t — Tests for CodingAdventures::Brainfuck::Lexer
# ============================================================================
#
# This test suite verifies that the grammar-driven Brainfuck lexer correctly
# tokenizes Brainfuck source code.
#
# Brainfuck has exactly 8 command tokens. Every other character is a comment
# and is silently consumed by the lexer's skip patterns. The token stream
# always ends with an EOF sentinel.
#
# Test categories:
#   - All 8 individual command tokens
#   - Comment skipping (various forms of non-command text)
#   - Empty source → just EOF
#   - Canonical program "++[>+<-]"
#   - Line/col tracking

use CodingAdventures::Brainfuck::Lexer;

# ============================================================================
# Helper: extract just the type fields from all non-EOF tokens.
# ============================================================================

sub token_types {
    my ($tokens) = @_;
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Test 1: RIGHT token ">"
# ============================================================================

subtest 'right token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('>');
    my $types = token_types($tokens);
    is $types, ['RIGHT'], '">" produces a RIGHT token';
    is $tokens->[0]{value}, '>', 'value is ">"';
};

# ============================================================================
# Test 2: LEFT token "<"
# ============================================================================

subtest 'left token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('<');
    my $types = token_types($tokens);
    is $types, ['LEFT'], '"<" produces a LEFT token';
};

# ============================================================================
# Test 3: INC token "+"
# ============================================================================

subtest 'inc token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('+');
    my $types = token_types($tokens);
    is $types, ['INC'], '"+" produces an INC token';
};

# ============================================================================
# Test 4: DEC token "-"
# ============================================================================

subtest 'dec token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('-');
    my $types = token_types($tokens);
    is $types, ['DEC'], '"-" produces a DEC token';
};

# ============================================================================
# Test 5: OUTPUT token "."
# ============================================================================

subtest 'output token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('.');
    my $types = token_types($tokens);
    is $types, ['OUTPUT'], '"." produces an OUTPUT token';
};

# ============================================================================
# Test 6: INPUT token ","
# ============================================================================

subtest 'input token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize(',');
    my $types = token_types($tokens);
    is $types, ['INPUT'], '"," produces an INPUT token';
};

# ============================================================================
# Test 7: LOOP_START token "["
# ============================================================================

subtest 'loop_start token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('[');
    my $types = token_types($tokens);
    is $types, ['LOOP_START'], '"[" produces a LOOP_START token';
};

# ============================================================================
# Test 8: LOOP_END token "]"
# ============================================================================

subtest 'loop_end token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize(']');
    my $types = token_types($tokens);
    is $types, ['LOOP_END'], '"]" produces a LOOP_END token';
};

# ============================================================================
# Test 9: Comment skipping — inline prose
# ============================================================================
# Brainfuck programs commonly embed documentation as comment text.
# "++ increment twice" should produce only INC INC, with the prose skipped.

subtest 'comment skipping inline prose' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('+ increment + again');
    my $types = token_types($tokens);
    is $types, ['INC', 'INC'], 'Inline prose comments are silently skipped';
};

# ============================================================================
# Test 10: Comment skipping — pure comment
# ============================================================================
# A string with no command characters should produce only EOF.

subtest 'comment only input' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('hello world this is all a comment');
    my $types = token_types($tokens);
    is $types, [], 'Comment-only input produces no command tokens';
    is $tokens->[-1]{type}, 'EOF', 'Last token is always EOF';
};

# ============================================================================
# Test 11: Empty source → just EOF
# ============================================================================

subtest 'empty source' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('');
    is scalar(@$tokens), 1, 'Empty source produces exactly one token';
    is $tokens->[0]{type}, 'EOF', 'That token is EOF';
};

# ============================================================================
# Test 12: Canonical program "++[>+<-]"
# ============================================================================
# The classic "copy cell 0 to cell 1 while zeroing cell 0" pattern.
# Expected token sequence: INC INC LOOP_START RIGHT INC LEFT DEC LOOP_END

subtest 'canonical ++[>+<-]' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('++[>+<-]');
    my $types = token_types($tokens);
    is $types,
       ['INC', 'INC', 'LOOP_START', 'RIGHT', 'INC', 'LEFT', 'DEC', 'LOOP_END'],
       'Canonical program tokenizes correctly';

    # Check that the last token is always EOF.
    is $tokens->[-1]{type}, 'EOF', 'Token stream ends with EOF';
};

# ============================================================================
# Test 13: Line and column tracking
# ============================================================================
# The very first token in "+" should be at line 1, col 1.

subtest 'line col tracking for first token' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('+');
    is $tokens->[0]{line}, 1, 'First token is on line 1';
    is $tokens->[0]{col},  1, 'First token is at col 1';
};

# ============================================================================
# Test 14: All 8 commands in sequence
# ============================================================================
# Tokenizing all 8 command characters together should produce all 8 tokens.

subtest 'all eight commands' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('><+-.,[]{');
    # Note: '{' is a comment (not a Brainfuck command), so it is skipped.
    my $types = token_types($tokens);
    is $types,
       ['RIGHT', 'LEFT', 'INC', 'DEC', 'OUTPUT', 'INPUT', 'LOOP_START', 'LOOP_END'],
       'All 8 command chars tokenize; { is a comment';
};

# ============================================================================
# Test 15: Whitespace between commands
# ============================================================================
# Spaces, tabs, and newlines between commands should be silently consumed.

subtest 'whitespace between commands' => sub {
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize("+ \t\n + \n\n -");
    my $types = token_types($tokens);
    is $types, ['INC', 'INC', 'DEC'], 'Whitespace between commands is consumed';
};

done_testing;
