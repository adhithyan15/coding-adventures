#!/usr/bin/env perl
# grammar-tools — validate and compile .tokens and .grammar files.
#
# This program wraps CodingAdventures::GrammarTools and
# CodingAdventures::GrammarTools::Compiler behind a small CLI. It is the
# Perl counterpart of the Ruby/Go/Rust/TypeScript/Elixir/Lua/Python
# grammar-tools CLIs in this monorepo — same commands, same exit codes.
#
# Usage
# -----
#
#     grammar-tools.pl validate <file.tokens> <file.grammar>
#     grammar-tools.pl validate-tokens <file.tokens>
#     grammar-tools.pl validate-grammar <file.grammar>
#     grammar-tools.pl compile-tokens <file.tokens> [-o <output.pm>]
#     grammar-tools.pl compile-grammar <file.grammar> [-o <output.pm>]
#
# Exit codes
# ----------
#
#   0  All checks passed / compilation succeeded.
#   1  One or more validation errors found / compile error.
#   2  Usage error (wrong number of arguments, unknown command).
#
# Compile commands
# -----------------
#
# The compile commands convert `.tokens`/`.grammar` files into Perl source
# code that embeds the grammar as native data structures, eliminating
# runtime file I/O and parsing in downstream packages. Unlike the Ruby/Go/
# Rust/TypeScript/Elixir ports, these commands do not run a validation
# step before compiling (matching the Lua port) — there is no `--force`
# flag because there is nothing to force past.

use strict;
use warnings;
use FindBin;
use File::Spec;

# ----------------------------------------------------------------------------
# Resolve the repo root so we can find code/packages/perl/grammar-tools/lib.
# Walk up from this file's directory until we find code/specs/grammar-tools.json.
# ----------------------------------------------------------------------------
sub find_root {
    my $dir = File::Spec->rel2abs($FindBin::Bin);
    for (1 .. 20) {
        my $marker = File::Spec->catfile($dir, 'code', 'specs', 'grammar-tools.json');
        return $dir if -e $marker;
        my $parent = File::Spec->catdir($dir, File::Spec->updir);
        my $abs_parent = File::Spec->rel2abs($parent);
        last if $abs_parent eq $dir;
        $dir = $abs_parent;
    }
    return File::Spec->rel2abs($FindBin::Bin);
}

my $ROOT = find_root();
use lib;
lib->import(File::Spec->catdir($ROOT, 'code', 'packages', 'perl', 'grammar-tools', 'lib'));

require CodingAdventures::GrammarTools;
require CodingAdventures::GrammarTools::Compiler;

# ----------------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------------

sub count_errors {
    my ($issues) = @_;
    my $n = 0;
    for my $issue (@$issues) {
        $n++ unless $issue =~ /^Warning:/;
    }
    return $n;
}

sub print_issues {
    my ($issues, $indent) = @_;
    $indent //= '  ';
    print STDERR "$indent$_\n" for @$issues;
}

sub basename {
    my ($path) = @_;
    my (undef, undef, $file) = File::Spec->splitpath($path);
    return $file;
}

sub read_file {
    my ($path) = @_;
    open my $fh, '<', $path or return (undef, "$!");
    local $/;
    my $content = <$fh>;
    close $fh;
    return ($content, undef);
}

# ----------------------------------------------------------------------------
# validate_command($tokens_path, $grammar_path)
# ----------------------------------------------------------------------------
sub validate_command {
    my ($tokens_path, $grammar_path) = @_;
    my $total_issues = 0;

    unless (-e $tokens_path) {
        print STDERR "Error: File not found: $tokens_path\n";
        exit 1;
    }
    print "Validating " . basename($tokens_path) . " ... ";

    my ($tsrc, $terr) = read_file($tokens_path);
    my ($token_grammar, $terr2) = CodingAdventures::GrammarTools->parse_token_grammar($tsrc);
    unless ($token_grammar) {
        print "PARSE ERROR\n";
        print "  $terr2\n";
        exit 1;
    }
    my $token_issues = CodingAdventures::GrammarTools->validate_token_grammar($token_grammar);
    my $token_errors = count_errors($token_issues);
    if ($token_errors > 0) {
        print "$token_errors error(s)\n";
        print_issues($token_issues);
        $total_issues += $token_errors;
    }
    else {
        my $n_tokens = scalar @{ $token_grammar->definitions };
        print "OK ($n_tokens tokens)\n";
    }

    unless (-e $grammar_path) {
        print STDERR "Error: File not found: $grammar_path\n";
        exit 1;
    }
    print "Validating " . basename($grammar_path) . " ... ";

    my ($gsrc, $gerr) = read_file($grammar_path);
    my ($parser_grammar, $gerr2) = CodingAdventures::GrammarTools->parse_parser_grammar($gsrc);
    unless ($parser_grammar) {
        print "PARSE ERROR\n";
        print "  $gerr2\n";
        exit 1;
    }
    my $token_names = $token_grammar->token_names;
    my $parser_issues = CodingAdventures::GrammarTools->validate_parser_grammar($parser_grammar, $token_names);
    my $parser_errors = count_errors($parser_issues);
    if ($parser_errors > 0) {
        print "$parser_errors error(s)\n";
        print_issues($parser_issues);
        $total_issues += $parser_errors;
    }
    else {
        my $n_rules = scalar @{ $parser_grammar->{rules} };
        print "OK ($n_rules rules)\n";
    }

    print "\n";
    if ($total_issues > 0) {
        print "Found $total_issues error(s). Fix them and try again.\n";
        return 1;
    }
    print "All checks passed.\n";
    return 0;
}

sub validate_tokens_only {
    my ($tokens_path) = @_;
    unless (-e $tokens_path) {
        print STDERR "Error: File not found: $tokens_path\n";
        exit 1;
    }
    print "Validating " . basename($tokens_path) . " ... ";
    my ($src, $err) = read_file($tokens_path);
    my ($grammar, $perr) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    unless ($grammar) {
        print "PARSE ERROR\n";
        print "  $perr\n";
        return 1;
    }
    my $issues = CodingAdventures::GrammarTools->validate_token_grammar($grammar);
    my $errors = count_errors($issues);
    if ($errors > 0) {
        print "$errors error(s)\n";
        print_issues($issues);
        print "\nFound $errors error(s). Fix them and try again.\n";
        return 1;
    }
    my $n = scalar @{ $grammar->definitions };
    print "OK ($n tokens)\n\nAll checks passed.\n";
    return 0;
}

sub validate_grammar_only {
    my ($grammar_path) = @_;
    unless (-e $grammar_path) {
        print STDERR "Error: File not found: $grammar_path\n";
        exit 1;
    }
    print "Validating " . basename($grammar_path) . " ... ";
    my ($src, $err) = read_file($grammar_path);
    my ($grammar, $perr) = CodingAdventures::GrammarTools->parse_parser_grammar($src);
    unless ($grammar) {
        print "PARSE ERROR\n";
        print "  $perr\n";
        return 1;
    }
    my $issues = CodingAdventures::GrammarTools->validate_parser_grammar($grammar);
    my $errors = count_errors($issues);
    if ($errors > 0) {
        print "$errors error(s)\n";
        print_issues($issues);
        print "\nFound $errors error(s). Fix them and try again.\n";
        return 1;
    }
    my $n = scalar @{ $grammar->{rules} };
    print "OK ($n rules)\n\nAll checks passed.\n";
    return 0;
}

# ----------------------------------------------------------------------------
# compile_tokens_command($tokens_path, $output_path)
# ----------------------------------------------------------------------------
sub compile_tokens_command {
    my ($tokens_path, $output_path) = @_;
    unless (-e $tokens_path) {
        print STDERR "Error: File not found: $tokens_path\n";
        return 1;
    }
    print STDERR "Compiling " . basename($tokens_path) . " ... ";
    my ($src, $err) = read_file($tokens_path);
    my ($grammar, $perr) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    unless ($grammar) {
        print STDERR "PARSE ERROR\n";
        print STDERR "  $perr\n";
        return 1;
    }
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($grammar, basename($tokens_path));
    if ($output_path) {
        open my $out, '>', $output_path or do {
            print STDERR "Error: cannot write '$output_path': $!\n";
            return 1;
        };
        print $out $code;
        close $out;
        print STDERR "OK -> $output_path\n";
    }
    else {
        print STDERR "OK\n";
        print $code;
    }
    return 0;
}

sub compile_grammar_command {
    my ($grammar_path, $output_path) = @_;
    unless (-e $grammar_path) {
        print STDERR "Error: File not found: $grammar_path\n";
        return 1;
    }
    print STDERR "Compiling " . basename($grammar_path) . " ... ";
    my ($src, $err) = read_file($grammar_path);
    my ($grammar, $perr) = CodingAdventures::GrammarTools->parse_parser_grammar($src);
    unless ($grammar) {
        print STDERR "PARSE ERROR\n";
        print STDERR "  $perr\n";
        return 1;
    }
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($grammar, basename($grammar_path));
    if ($output_path) {
        open my $out, '>', $output_path or do {
            print STDERR "Error: cannot write '$output_path': $!\n";
            return 1;
        };
        print $out $code;
        close $out;
        print STDERR "OK -> $output_path\n";
    }
    else {
        print STDERR "OK\n";
        print $code;
    }
    return 0;
}

# ----------------------------------------------------------------------------
# dispatch($command, \@files, $output_path)
# ----------------------------------------------------------------------------
sub dispatch {
    my ($command, $files, $output_path) = @_;

    if ($command eq 'validate') {
        if (@$files != 2) {
            print STDERR "Error: 'validate' requires two arguments: <tokens> <grammar>\n";
            return 2;
        }
        return validate_command($files->[0], $files->[1]);
    }
    if ($command eq 'validate-tokens') {
        if (@$files != 1) {
            print STDERR "Error: 'validate-tokens' requires one argument: <tokens>\n";
            return 2;
        }
        return validate_tokens_only($files->[0]);
    }
    if ($command eq 'validate-grammar') {
        if (@$files != 1) {
            print STDERR "Error: 'validate-grammar' requires one argument: <grammar>\n";
            return 2;
        }
        return validate_grammar_only($files->[0]);
    }
    if ($command eq 'compile-tokens') {
        if (@$files != 1) {
            print STDERR "Error: 'compile-tokens' requires one argument: <tokens>\n";
            return 2;
        }
        return compile_tokens_command($files->[0], $output_path);
    }
    if ($command eq 'compile-grammar') {
        if (@$files != 1) {
            print STDERR "Error: 'compile-grammar' requires one argument: <grammar>\n";
            return 2;
        }
        return compile_grammar_command($files->[0], $output_path);
    }

    print STDERR "Error: unknown command '$command'\n";
    return 2;
}

# ----------------------------------------------------------------------------
# main
# ----------------------------------------------------------------------------
sub main {
    my @args = @ARGV;
    unless (@args) {
        print STDERR "Usage: grammar-tools.pl <command> [args...]\n";
        return 2;
    }

    my $command = shift @args;
    my $output_path;
    my @files;
    while (@args) {
        my $arg = shift @args;
        if ($arg eq '-o' || $arg eq '--output') {
            $output_path = shift @args;
        }
        else {
            push @files, $arg;
        }
    }

    return dispatch($command, \@files, $output_path);
}

# `unless caller` — only run when executed directly, not when `require`d
# from a test file. Lets t/01-cli.t require this script and call main(),
# dispatch(), etc. as plain functions without an exit() killing the test
# process.
exit(main()) unless caller;

1;
