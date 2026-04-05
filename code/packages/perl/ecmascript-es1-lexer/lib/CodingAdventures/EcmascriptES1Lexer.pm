package CodingAdventures::EcmascriptES1Lexer;

# ============================================================================
# CodingAdventures::EcmascriptES1Lexer — Grammar-driven ECMAScript 1 tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools. It reads the shared `ecmascript/es1.tokens`
# grammar file, compiles the token definitions into Perl regexes, and applies
# them in priority order to tokenize ECMAScript 1 (1997) source code.
#
# # What is ECMAScript 1?
# =======================
#
# The first standardized version of JavaScript (ECMA-262, 1st Edition, June
# 1997). It defines 23 keywords, basic operators (no === or !==), string and
# number literals, and the $ character as valid in identifiers.
#
# ES1 does NOT have: strict equality, try/catch, regex literals, let/const,
# class, arrow functions, or template literals.
#
# # Architecture
# ==============
#
# 1. Grammar loading — reads `ecmascript/es1.tokens` and caches it.
# 2. Pattern compilation — converts definitions into qr/\G.../ patterns.
# 3. Tokenization — walks source using \G + pos() mechanism.
#
# # Path navigation
# =================
#
# `__FILE__` = lib/CodingAdventures/EcmascriptES1Lexer.pm
#   ↑ up 1 → lib/
#   ↑ up 2 → ecmascript-es1-lexer/
#   ↑ up 3 → perl/
#   ↑ up 4 → packages/
#   ↑ up 5 → code/
# + /grammars/ecmascript/es1.tokens
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

my $_grammar;
my $_rules;
my $_skip_rules;
my $_keyword_map;

sub _grammars_dir {
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'ecmascript', 'es1.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::EcmascriptES1Lexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::EcmascriptES1Lexer: failed to parse es1.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    return $_grammar;
}

sub _build_rules {
    return if $_rules;

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    for my $defn ( @{ $grammar->skip_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        push @skip_rules, $pat;
    }

    for my $defn ( @{ $grammar->definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        my $type = ( $defn->alias && $defn->alias ne '' )
                    ? $defn->alias
                    : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    unless (@skip_rules) {
        push @skip_rules, qr/\G[ \t\r\n]+/;
    }

    my %kw_map;
    $kw_map{$_} = uc($_) for @{ $grammar->keywords };
    $_keyword_map = \%kw_map;

    $_skip_rules = \@skip_rules;
    $_rules      = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

sub tokenize {
    my ($class_or_self, $source) = @_;

    _build_rules();

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;
                my $nl_count = () = $matched =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    my $after_last_nl = $matched;
                    $after_last_nl =~ s/.*\n//s;
                    $col = length($after_last_nl) + 1;
                } else {
                    $col += length($matched);
                }
                $pos = pos($source);
                $skipped = 1;
                last;
            }
        }
        next if $skipped;

        my $matched_tok = 0;
        for my $rule (@$_rules) {
            pos($source) = $pos;
            if ($source =~ /$rule->{pat}/gc) {
                my $value = $&;

                my $tok_type = $rule->{name};
                if ($tok_type eq 'NAME' && exists $_keyword_map->{$value}) {
                    $tok_type = $_keyword_map->{$value};
                }
                push @tokens, {
                    type  => $tok_type,
                    value => $value,
                    line  => $line,
                    col   => $col,
                };

                $pos = pos($source);

                my $nl_count = () = $value =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    my $after_last_nl = $value;
                    $after_last_nl =~ s/.*\n//s;
                    $col = length($after_last_nl) + 1;
                } else {
                    $col += length($value);
                }

                $matched_tok = 1;
                last;
            }
        }

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::EcmascriptES1Lexer: LexerError at line %d col %d: "
              . "unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };

    return \@tokens;
}

1;

__END__

=head1 NAME

CodingAdventures::EcmascriptES1Lexer - Grammar-driven ECMAScript 1 (1997) tokenizer

=head1 SYNOPSIS

    use CodingAdventures::EcmascriptES1Lexer;

    my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('var x = 1;');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

Tokenizes ECMAScript 1 source code using the shared es1.tokens grammar file.

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
