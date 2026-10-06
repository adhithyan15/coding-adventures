package CodingAdventures::BuildTool::GlobMatch;

# GlobMatch.pm -- Glob Pattern Matching for Starlark srcs Lists
# ==============================================================
#
# Starlark BUILD files use glob patterns to list source files:
#
#   perl_library(
#     name = "logic-gates",
#     srcs = glob(["lib/**/*.pm", "t/**/*.t"]),
#   )
#
# This module converts glob patterns to Perl regular expressions, then
# matches files against them.
#
# Glob pattern rules:
#   *       -- matches any characters except '/'
#   **      -- matches any characters including '/'
#   ?       -- matches any single character except '/'
#   [abc]   -- matches one of: a, b, c
#   [^abc]  -- matches any character not in: a, b, c
#
# Examples:
#   "lib/*.pm"       matches "lib/Foo.pm", not "lib/a/b.pm"
#   "lib/**/*.pm"    matches "lib/Foo.pm" AND "lib/a/b.pm"
#   "t/??.t"         matches "t/00.t", "t/99.t"
#
# Perl advantages demonstrated here:
#   - Regex as first-class values: `my $re = qr/$pattern/`.
#   - s/// with /e modifier for dynamic transformations.
#   - Regex character classes map directly to glob character classes.

use strict;
use warnings;

our $VERSION = '0.01';

# new -- Constructor.
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# glob_to_regex -- Convert a glob pattern string to a compiled Perl regex.
#
# The conversion rules:
#
#   Glob     | Regex
#   ---------|------
#   **       | .*          (any chars including /)
#   *        | [^/]*       (any chars except /)
#   ?        | [^/]        (one char except /)
#   [...]    | [...]       (character class, unchanged)
#   .        | \.          (literal dot)
#   other    | same        (literal)
#
# We handle ** before * to prevent double-processing.
#
# @param $glob -- Glob pattern string.
# @return compiled qr// regex.
sub glob_to_regex {
    my ($self, $glob) = @_;

    # We build the regex by scanning the glob character by character rather
    # than using quotemeta + substitution, which is safer and more readable.
    #
    # Conversion rules:
    #   **/  at the start or after /  →  (?:.*/)?   (zero or more path components)
    #   **   elsewhere                →  .*          (any chars)
    #   *                             →  [^/]*       (any chars except /)
    #   ?                             →  [^/]        (one char except /)
    #   [...]                         →  [...]       (character class, unchanged)
    #   .                             →  \.          (literal dot)
    #   other                         →  \Q$char\E   (literal)

    my $pattern = '';
    my @chars   = split //, $glob;
    my $i       = 0;

    while ($i < @chars) {
        my $c = $chars[$i];

        if ($c eq '*') {
            if ($i + 1 < @chars && $chars[$i + 1] eq '*') {
                # Double star: **
                $i += 2;
                if ($i < @chars && $chars[$i] eq '/') {
                    # **/ — matches any number of path components (including zero)
                    $pattern .= '(?:.*/)?';
                    $i++;  # consume the /
                } else {
                    # ** not followed by / — matches anything
                    $pattern .= '.*';
                }
            } else {
                # Single star — matches anything except /
                $pattern .= '[^/]*';
                $i++;
            }
        } elsif ($c eq '?') {
            $pattern .= '[^/]';
            $i++;
        } elsif ($c eq '[') {
            # Character class — Python fnmatchcase treats an unmatched '[' as
            # a literal. A leading '!' negates the class; Perl spells that '^'.
            my $j = $i + 1;
            my $negated = $j < @chars && $chars[$j] eq '!';
            $j++ if $negated || ($j < @chars && $chars[$j] eq '^');
            $j++ if $j < @chars && $chars[$j] eq ']';  # handle literal ] at start
            while ($j < @chars && $chars[$j] ne ']') { $j++ }
            if ($j >= @chars) {
                $pattern .= '\\[';
                $i++;
            } else {
                my $class = join('', @chars[$i..$j]);
                $class =~ s/^\[!/[^/ if $negated;
                $pattern .= $class;
                $i = $j + 1;
            }
        } elsif ($c eq '.') {
            $pattern .= '\\.';
            $i++;
        } else {
            $pattern .= quotemeta($c);
            $i++;
        }
    }

    return qr/^$pattern$/;
}

# matches -- Test whether a file path matches a glob pattern.
#
# @param $glob -- Glob pattern string.
# @param $path -- File path (relative or absolute).
# @return 1 if matches, 0 if not.
sub matches {
    my ($self, $glob, $path) = @_;
    my $re = $self->glob_to_regex($glob);
    return ($path =~ $re) ? 1 : 0;
}

# filter_files -- Return the subset of files matching any of the given globs.
#
# @param \@globs -- List of glob pattern strings.
# @param \@files -- List of file paths to filter.
# @return list of matching file paths.
sub filter_files {
    my ($self, $globs_ref, $files_ref) = @_;
    my @patterns = map { $self->glob_to_regex($_) } @{$globs_ref};
    return grep {
        my $file = $_;
        grep { $file =~ $_ } @patterns
    } @{$files_ref};
}

# The portable source selector deliberately does not use the regex converter
# above. Repeated globstars give a backtracking regex exponentially many ways
# to fail, and a negated regex class may consume a path separator. Compile
# slash-free segments once and match them with bounded state grids instead.
sub compile_portable {
    my ($self, $glob) = @_;
    my @segments;
    for my $segment (split m{/}, $glob, -1) {
        if ($segment eq '**') {
            push @segments, undef;
            next;
        }
        my @chars = split //, $segment;
        my @tokens;
        for (my $i = 0; $i < @chars;) {
            my $char = $chars[$i];
            if ($char eq '*') {
                push @tokens, ['star'] unless @tokens && $tokens[-1][0] eq 'star';
                $i++;
            } elsif ($char eq '?') {
                push @tokens, ['question'];
                $i++;
            } elsif ($char ne '[') {
                push @tokens, ['literal', $char];
                $i++;
            } else {
                my $cursor = $i + 1;
                my $negated = $cursor < @chars && $chars[$cursor] eq '!';
                $cursor++ if $negated;
                my $closing = $cursor;
                $closing++ if $closing < @chars && $chars[$closing] eq ']';
                $closing++ while $closing < @chars && $chars[$closing] ne ']';
                if ($closing >= @chars) {
                    push @tokens, ['literal', '['];
                    $i++;
                    next;
                }
                my $body = join('', @chars[$cursor .. $closing - 1]);
                die "DECLARED_GLOB_INVALID: ambiguous class\n"
                    if $body =~ /(?:--|&&|~~|\|\|)/;
                my @members;
                while ($cursor < $closing) {
                    if ($cursor + 2 < $closing && $chars[$cursor + 1] eq '-') {
                        die "DECLARED_GLOB_INVALID: descending class\n"
                            if ord($chars[$cursor]) > ord($chars[$cursor + 2]);
                        push @members, [$chars[$cursor], $chars[$cursor + 2]];
                        $cursor += 3;
                    } else {
                        push @members, $chars[$cursor];
                        $cursor++;
                    }
                }
                push @tokens, ['class', $negated, \@members];
                $i = $closing + 1;
            }
        }
        push @segments, \@tokens;
    }
    return \@segments;
}

sub matches_portable_compiled {
    my ($self, $compiled, $path) = @_;
    my @parts = split m{/}, $path, -1;
    my $end = scalar @parts;
    my @next = (0) x ($end + 1);
    $next[$end] = 1;
    for my $segment (reverse @$compiled) {
        my @row = (0) x ($end + 1);
        if (!defined $segment) {
            $row[$end] = $next[$end];
            for (my $i = $end - 1; $i >= 0; $i--) {
                $row[$i] = $next[$i] || $row[$i + 1];
            }
        } else {
            for (my $i = $end - 1; $i >= 0; $i--) {
                $row[$i] = $next[$i + 1] && _match_portable_segment($segment, $parts[$i]);
            }
        }
        @next = @row;
    }
    return !!$next[0];
}

sub _match_portable_segment {
    my ($tokens, $value) = @_;
    my @chars = split //, $value;
    my $end = scalar @chars;
    my @next = (0) x ($end + 1);
    $next[$end] = 1;
    for my $token (reverse @$tokens) {
        my @row = (0) x ($end + 1);
        my $type = $token->[0];
        if ($type eq 'star') {
            $row[$end] = $next[$end];
            for (my $i = $end - 1; $i >= 0; $i--) {
                $row[$i] = $next[$i] || $row[$i + 1];
            }
        } else {
            for (my $i = $end - 1; $i >= 0; $i--) {
                $row[$i] = $next[$i + 1] && (
                    $type eq 'question' ||
                    ($type eq 'literal' && $token->[1] eq $chars[$i]) ||
                    ($type eq 'class' && _class_has($token, $chars[$i]))
                );
            }
        }
        @next = @row;
    }
    return !!$next[0];
}

sub _class_has {
    my ($token, $char) = @_;
    my $matched = 0;
    for my $member (@{ $token->[2] }) {
        if (ref($member) eq 'ARRAY') {
            $matched ||= $member->[0] le $char && $char le $member->[1];
        } else {
            $matched ||= $member eq $char;
        }
    }
    return $token->[1] ? !$matched : $matched;
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::GlobMatch - Glob pattern matching for Starlark srcs

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::GlobMatch;

  my $gm = CodingAdventures::BuildTool::GlobMatch->new();
  my $re = $gm->glob_to_regex("lib/**/*.pm");

  my @matching = $gm->filter_files(
      ["lib/**/*.pm", "t/**/*.t"],
      \@all_files,
  );

=head1 DESCRIPTION

Converts glob patterns to Perl regular expressions for matching source file
lists in Starlark BUILD rules.

=cut
