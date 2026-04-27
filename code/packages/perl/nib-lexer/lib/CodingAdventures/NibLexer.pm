package CodingAdventures::NibLexer;

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
    my $dir = File::Spec->rel2abs(dirname(__FILE__));
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile(_grammars_dir(), 'nib.tokens');
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::NibLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::NibLexer: failed to parse nib.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    return $_grammar;
}

sub _build_rules {
    return if $_rules;

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    for my $defn (@{ $grammar->skip_definitions }) {
        my $pat = $defn->is_regex
            ? qr/\G(?:${\$defn->pattern})/
            : qr/\G\Q@{[$defn->pattern]}\E/;
        push @skip_rules, $pat;
    }

    for my $defn (@{ $grammar->definitions }) {
        my $pat = $defn->is_regex
            ? qr/\G(?:${\$defn->pattern})/
            : qr/\G\Q@{[$defn->pattern]}\E/;
        my $type = ($defn->alias && $defn->alias ne '') ? $defn->alias : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    my %kw_map;
    $kw_map{$_} = uc($_) for @{ $grammar->keywords };

    $_skip_rules = \@skip_rules;
    $_rules = \@rules;
    $_keyword_map = \%kw_map;
}

sub tokenize {
    my ($class, $source) = @_;

    _build_rules();

    my @tokens;
    my $pos = 0;
    my $line = 1;
    my $col = 1;
    my $len = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;
                my $newlines = ($matched =~ tr/\n//);
                if ($newlines > 0) {
                    $line += $newlines;
                    my ($tail) = $matched =~ /([^\n]*)\z/;
                    $col = length($tail) + 1;
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
                my $type = $rule->{name};
                if ($type eq 'NAME' && exists $_keyword_map->{$value}) {
                    $type = $_keyword_map->{$value};
                }
                push @tokens, {
                    type => $type,
                    value => $value,
                    line => $line,
                    col => $col,
                };

                my $newlines = ($value =~ tr/\n//);
                if ($newlines > 0) {
                    $line += $newlines;
                    my ($tail) = $value =~ /([^\n]*)\z/;
                    $col = length($tail) + 1;
                } else {
                    $col += length($value);
                }
                $pos = pos($source);
                $matched_tok = 1;
                last;
            }
        }

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::NibLexer: LexerError at line %d col %d: unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };
    return \@tokens;
}

1;
