package CodingAdventures::AsciidocParser;

# ============================================================================
# CodingAdventures::AsciidocParser — Document AST parser wrapper for the Perl AsciiDoc subset
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
##
# The older `CodingAdventures::Asciidoc` package parses AsciiDoc and can render
# HTML directly. This package provides the parser-shaped surface used by the
# rest of the repo: `parse($source)` returns a document AST root with
# `children`, and all inline links/images use the shared `destination` field.
#
# Usage:
#
#   use CodingAdventures::AsciidocParser;
#   my $doc = CodingAdventures::AsciidocParser::parse("= Hello\n\nWorld\n");
#
# ============================================================================

use strict;
use warnings;
use lib '../asciidoc/lib';

our $VERSION = '0.01';

use CodingAdventures::Asciidoc;

sub parse {
    my ($text) = @_;
    my $blocks = CodingAdventures::Asciidoc::parse($text // '');
    return {
        type     => 'document',
        children => [ map { _convert_block($_) } @$blocks ],
    };
}

sub _convert_block {
    my ($node) = @_;
    my $type = $node->{type} // '';

    if ($type eq 'heading') {
        return {
            type     => 'heading',
            level    => $node->{level},
            children => _convert_inlines($node->{children}),
        };
    }

    if ($type eq 'paragraph') {
        return {
            type     => 'paragraph',
            children => _convert_inlines($node->{children}),
        };
    }

    if ($type eq 'code_block') {
        my $language = $node->{language};
        $language = undef if defined($language) && $language eq '';
        return {
            type     => 'code_block',
            language => $language,
            value    => $node->{value} // '',
        };
    }

    if ($type eq 'blockquote') {
        return {
            type     => 'blockquote',
            children => [ map { _convert_block($_) } @{ $node->{children} // [] } ],
        };
    }

    if ($type eq 'list') {
        return {
            type     => 'list',
            ordered  => $node->{ordered} ? 1 : 0,
            children => [
                map {
                    {
                        type     => 'list_item',
                        children => [
                            {
                                type     => 'paragraph',
                                children => _convert_inlines($_),
                            }
                        ],
                    }
                } @{ $node->{items} // [] }
            ],
        };
    }

    if ($type eq 'thematic_break') {
        return { type => 'thematic_break' };
    }

    if ($type eq 'raw_block') {
        return {
            type  => 'raw_block',
            value => $node->{value} // '',
        };
    }

    return { %$node };
}

sub _convert_inlines {
    my ($nodes) = @_;
    return [ map { _convert_inline($_) } @{ $nodes // [] } ];
}

sub _convert_inline {
    my ($node) = @_;
    my $type = $node->{type} // '';

    if ($type eq 'strong') {
        return {
            type     => 'strong',
            children => _convert_inlines($node->{children}),
        };
    }

    if ($type eq 'emph') {
        return {
            type     => 'emphasis',
            children => _convert_inlines($node->{children}),
        };
    }

    if ($type eq 'link') {
        return {
            type        => 'link',
            destination => $node->{href} // '',
            children    => _convert_inlines($node->{children}),
        };
    }

    if ($type eq 'image') {
        return {
            type        => 'image',
            destination => $node->{src} // '',
            alt         => $node->{alt} // '',
        };
    }

    if ($type eq 'text' || $type eq 'code_span') {
        return {
            type  => $type,
            value => $node->{value} // '',
        };
    }

    if ($type eq 'hard_break' || $type eq 'soft_break') {
        return { type => $type };
    }

    return { %$node };
}

1;

__END__

=head1 NAME

CodingAdventures::AsciidocParser - Document AST parser wrapper for the Perl AsciiDoc subset

=head1 SYNOPSIS

    use CodingAdventures::AsciidocParser;
    my $doc = CodingAdventures::AsciidocParser::parse("= Hello\n");

=head1 DESCRIPTION

Document AST parser wrapper for the Perl AsciiDoc subset

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
