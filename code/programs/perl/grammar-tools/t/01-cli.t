#!/usr/bin/env perl
# t/01-cli.t — Tests for grammar-tools.pl's dispatch/compile/validate logic.
#
# grammar-tools.pl is a plain script (not a .pm module), so we `require` it
# directly. Its final line is `exit(main()) unless caller;` — since this
# file `require`s it (giving it a caller), main() does NOT run automatically
# and does NOT exit our test process. We then call dispatch(), main::main(),
# etc. as plain functions defined in the `main` package.

use strict;
use warnings;
use FindBin qw($Bin);
use File::Spec;
use File::Temp qw(tempfile);

use Test2::V0;

require File::Spec->catfile($Bin, File::Spec->updir, 'grammar-tools.pl');

my $ROOT = File::Spec->rel2abs(File::Spec->catdir($Bin, ('..') x 5));
my $GRAMMARS_DIR = File::Spec->catdir($ROOT, 'code', 'grammars');
my $JSON_TOKENS = File::Spec->catfile($GRAMMARS_DIR, 'json', 'json.tokens');
my $JSON_GRAMMAR = File::Spec->catfile($GRAMMARS_DIR, 'json', 'json.grammar');

ok(-e $JSON_TOKENS, "sanity: json.tokens exists at $JSON_TOKENS");
ok(-e $JSON_GRAMMAR, "sanity: json.grammar exists at $JSON_GRAMMAR");

# ============================================================================
# dispatch() — argument-count / usage-error paths
# ============================================================================

subtest 'dispatch: usage errors return 2' => sub {
    is(main::dispatch('validate', [], undef), 2, 'validate with no files');
    is(main::dispatch('validate', [$JSON_TOKENS], undef), 2, 'validate with one file');
    is(main::dispatch('validate-tokens', [], undef), 2, 'validate-tokens with no files');
    is(main::dispatch('validate-grammar', [], undef), 2, 'validate-grammar with no files');
    is(main::dispatch('compile-tokens', [], undef), 2, 'compile-tokens with no files');
    is(main::dispatch('compile-grammar', [], undef), 2, 'compile-grammar with no files');
    is(main::dispatch('bogus-command', [$JSON_TOKENS], undef), 2, 'unknown command');
};

# ============================================================================
# dispatch() — happy paths
# ============================================================================

subtest 'dispatch: validate succeeds on json grammar pair' => sub {
    is(main::dispatch('validate', [$JSON_TOKENS, $JSON_GRAMMAR], undef), 0);
};

subtest 'dispatch: validate-tokens succeeds on json.tokens' => sub {
    is(main::dispatch('validate-tokens', [$JSON_TOKENS], undef), 0);
};

subtest 'dispatch: validate-grammar succeeds on json.grammar' => sub {
    is(main::dispatch('validate-grammar', [$JSON_GRAMMAR], undef), 0);
};

subtest 'dispatch: compile-tokens succeeds on json.tokens' => sub {
    is(main::dispatch('compile-tokens', [$JSON_TOKENS], undef), 0);
};

subtest 'dispatch: compile-grammar succeeds on json.grammar' => sub {
    is(main::dispatch('compile-grammar', [$JSON_GRAMMAR], undef), 0);
};

# ============================================================================
# compile_tokens_command() — output file writing + executability
# ============================================================================

subtest 'compile_tokens_command: writes output file when path given' => sub {
    my (undef, $out_path) = tempfile(SUFFIX => '.pm', UNLINK => 1);
    my $result = main::compile_tokens_command($JSON_TOKENS, $out_path);
    is($result, 0, 'returns 0');

    open my $fh, '<', $out_path or die "cannot read $out_path: $!";
    local $/;
    my $content = <$fh>;
    close $fh;

    like($content, qr/DO NOT EDIT/, 'has generated-file header');
    like($content, qr/sub token_grammar/, 'defines token_grammar sub');
    like($content, qr/TokenGrammar/, 'blesses into TokenGrammar');
};

subtest 'compile_tokens_command: generated code is require-able and executable' => sub {
    my (undef, $out_path) = tempfile(SUFFIX => '.pm', UNLINK => 1);
    is(main::compile_tokens_command($JSON_TOKENS, $out_path), 0);

    my $pkg = 'Test::Generated::JsonTokens';
    my $wrapped = "package $pkg;\n" . "require '$out_path';\n" . "1;\n";
    ok(eval $wrapped, "generated code evaluates cleanly: $@") or diag($@);

    no strict 'refs';
    my $grammar = &{"${pkg}::token_grammar"}();
    is(ref($grammar), 'CodingAdventures::GrammarTools::TokenGrammar', 'returns a blessed TokenGrammar');
    ok(scalar(@{ $grammar->definitions }) > 0, 'has at least one token definition');
};

subtest 'compile_grammar_command: generated code is require-able and executable' => sub {
    my (undef, $out_path) = tempfile(SUFFIX => '.pm', UNLINK => 1);
    is(main::compile_grammar_command($JSON_GRAMMAR, $out_path), 0);

    my $pkg = 'Test::Generated::JsonGrammar';
    my $wrapped = "package $pkg;\n" . "require '$out_path';\n" . "1;\n";
    ok(eval $wrapped, "generated code evaluates cleanly: $@") or diag($@);

    no strict 'refs';
    my $grammar = &{"${pkg}::parser_grammar"}();
    is(ref($grammar), 'HASH', 'returns a plain hashref');
    ok(scalar(@{ $grammar->{rules} }) > 0, 'has at least one rule');
};

subtest 'compile_tokens_command: returns 1 on missing file' => sub {
    is(main::compile_tokens_command('/nonexistent/path/x.tokens', undef), 1);
};

subtest 'compile_grammar_command: returns 1 on missing file' => sub {
    is(main::compile_grammar_command('/nonexistent/path/x.grammar', undef), 1);
};

# ============================================================================
# main() — full @ARGV-driven entry point, including -o flag parsing
# ============================================================================

subtest 'main: no arguments returns usage error' => sub {
    local @ARGV = ();
    is(main::main(), 2);
};

subtest 'main: -o flag is parsed and honored' => sub {
    my (undef, $out_path) = tempfile(SUFFIX => '.pm', UNLINK => 1);
    local @ARGV = ('compile-tokens', $JSON_TOKENS, '-o', $out_path);
    is(main::main(), 0);
    ok(-s $out_path, 'output file was written and is non-empty');
};

subtest 'main: -p flag emits a package declaration and is require-able by path' => sub {
    my (undef, $out_path) = tempfile(SUFFIX => '.pm', UNLINK => 1);
    local @ARGV = ('compile-tokens', $JSON_TOKENS, '-o', $out_path, '-p', 'Test::Cli::JsonGrammar');
    is(main::main(), 0);

    open my $fh, '<', $out_path or die "cannot read $out_path: $!";
    local $/;
    my $content = <$fh>;
    close $fh;
    like($content, qr/^package Test::Cli::JsonGrammar;/m, 'package line present');

    require $out_path;
    no strict 'refs';
    my $grammar = &{'Test::Cli::JsonGrammar::token_grammar'}();
    is(ref($grammar), 'CodingAdventures::GrammarTools::TokenGrammar', 'require-able by file path, callable by qualified name');
};

done_testing;
