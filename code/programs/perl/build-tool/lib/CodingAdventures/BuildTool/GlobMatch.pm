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
            # Character class — find the matching ] and pass through verbatim.
            my $j = $i + 1;
            $j++ if $j < @chars && $chars[$j] eq '^';  # handle negation [^...]
            $j++ if $j < @chars && $chars[$j] eq ']';  # handle literal ] at start
            while ($j < @chars && $chars[$j] ne ']') { $j++ }
            $pattern .= join('', @chars[$i..$j]);
            $i = $j + 1;
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
