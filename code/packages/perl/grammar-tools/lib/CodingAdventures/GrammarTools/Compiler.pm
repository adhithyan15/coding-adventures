package CodingAdventures::GrammarTools::Compiler;

# ============================================================================
# CodingAdventures::GrammarTools::Compiler — compile parsed grammars to Perl
# ============================================================================
#
# The grammar-tools library parses `.tokens` and `.grammar` files into
# in-memory TokenGrammar / ParserGrammar objects. This module adds the
# *compile* step: given a parsed grammar object, generate Perl source code
# that reconstructs the same object graph as a literal data structure —
# eliminating all file I/O and parse overhead at runtime.
#
# This mirrors the Ruby/Go/Rust/TypeScript/Elixir/Lua/Python compilers in
# this monorepo. Generated output shape (ruby.tokens -> Grammar.pm):
#
#   package CodingAdventures::RubyLexer::Grammar;
#   # AUTO-GENERATED FILE — DO NOT EDIT
#   use strict;
#   use warnings;
#   sub token_grammar {
#       return bless {
#           definitions => [ bless({ name => 'NAME', ... },
#               'CodingAdventures::GrammarTools::TokenDefinition'), ... ],
#           ...
#       }, 'CodingAdventures::GrammarTools::TokenGrammar';
#   }
#   1;
#
# TokenGrammar/TokenDefinition/PatternGroup are blessed objects (the runtime
# lexer code calls accessor methods like `$grammar->skip_definitions` and
# `$defn->pattern`), so the compiler re-blesses literal hashrefs into the
# same packages. ParserGrammar is a plain hashref (`{ rules => [...],
# version => 0 }`) with plain nested element hashrefs (`{ type => ..., ... }`).

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# _perl_string($str) — render a Perl value as a single-quoted string literal.
#
# Single-quoted strings only need '\' and "'" escaped (no interpolation of
# $ or @, which regex patterns are full of). undef renders as 'undef'.
# ----------------------------------------------------------------------------
sub _perl_string {
    my ($str) = @_;
    return 'undef' unless defined $str;
    my $s = $str;
    $s =~ s/\\/\\\\/g;
    $s =~ s/'/\\'/g;
    return "'$s'";
}

sub _perl_bool {
    my ($val) = @_;
    return $val ? '1' : '0';
}

# ----------------------------------------------------------------------------
# _string_list_src(\@strings, $indent) — render an arrayref of strings.
# ----------------------------------------------------------------------------
sub _string_list_src {
    my ($list, $indent) = @_;
    return '[]' unless @$list;
    my $inner = $indent . '    ';
    my $items = join(",\n", map { "$inner" . _perl_string($_) } @$list);
    return "[\n$items,\n$indent]";
}

# ----------------------------------------------------------------------------
# _token_definition_src($defn, $indent) — render one TokenDefinition.
# ----------------------------------------------------------------------------
sub _token_definition_src {
    my ($defn, $indent) = @_;
    my $i = $indent . '    ';
    return "${indent}bless({\n"
        . "${i}name => " . _perl_string($defn->name) . ",\n"
        . "${i}pattern => " . _perl_string($defn->pattern) . ",\n"
        . "${i}is_regex => " . _perl_bool($defn->is_regex) . ",\n"
        . "${i}line_number => " . $defn->line_number . ",\n"
        . "${i}alias => " . _perl_string($defn->alias) . ",\n"
        . "${indent}}, 'CodingAdventures::GrammarTools::TokenDefinition')";
}

sub _token_def_list_src {
    my ($defs, $indent) = @_;
    return '[]' unless @$defs;
    my $inner = $indent . '    ';
    my $items = join(",\n", map { _token_definition_src($_, $inner) } @$defs);
    return "[\n$items,\n$indent]";
}

sub _groups_src {
    my ($groups, $indent) = @_;
    my @names = sort keys %$groups;
    return '{}' unless @names;
    my $inner = $indent . '    ';
    my $inner2 = $inner . '    ';
    my @entries;
    for my $name (@names) {
        my $group = $groups->{$name};
        my $defs_src = _token_def_list_src($group->definitions, $inner2 . '    ');
        push @entries,
            "$inner" . _perl_string($name) . " => bless({\n"
            . "${inner2}name => " . _perl_string($group->name) . ",\n"
            . "${inner2}definitions => $defs_src,\n"
            . "$inner}, 'CodingAdventures::GrammarTools::PatternGroup')";
    }
    return "{\n" . join(",\n", @entries) . ",\n$indent}";
}

# ----------------------------------------------------------------------------
# compile_token_grammar($grammar, $source_file) -> Perl source string
# ----------------------------------------------------------------------------
sub compile_token_grammar {
    my ($grammar, $source_file) = @_;
    $source_file //= '';

    my $defs_src        = _token_def_list_src($grammar->definitions, '        ');
    my $skip_src        = _token_def_list_src($grammar->skip_definitions, '        ');
    my $err_src         = _token_def_list_src($grammar->error_definitions, '        ');
    my $groups_src      = _groups_src($grammar->groups, '        ');
    my $keywords_src    = _string_list_src($grammar->keywords, '        ');
    my $ctx_kw_src      = _string_list_src($grammar->context_keywords, '        ');
    my $layout_kw_src   = _string_list_src($grammar->layout_keywords, '        ');
    my $soft_kw_src     = _string_list_src($grammar->soft_keywords, '        ');
    my $reserved_kw_src = _string_list_src($grammar->reserved_keywords, '        ');
    my $transitions_src = _string_list_src($grammar->transitions, '        ');
    my $mode_src        = _perl_string($grammar->mode);
    my $escape_mode_src = _perl_string($grammar->escape_mode);
    my $start_mode_src  = _perl_string($grammar->start_mode);

    my $source_line = $source_file eq '' ? '' : "# Source: $source_file\n";

    my $body = "sub token_grammar {\n"
        . "    return bless {\n"
        . "        definitions => $defs_src,\n"
        . "        keywords => $keywords_src,\n"
        . "        context_keywords => $ctx_kw_src,\n"
        . "        layout_keywords => $layout_kw_src,\n"
        . "        soft_keywords => $soft_kw_src,\n"
        . "        mode => $mode_src,\n"
        . "        escape_mode => $escape_mode_src,\n"
        . "        skip_definitions => $skip_src,\n"
        . "        error_definitions => $err_src,\n"
        . "        reserved_keywords => $reserved_kw_src,\n"
        . "        groups => $groups_src,\n"
        . "        start_mode => $start_mode_src,\n"
        . "        transitions => $transitions_src,\n"
        . "    }, 'CodingAdventures::GrammarTools::TokenGrammar';\n"
        . "}\n";

    return "# AUTO-GENERATED FILE — DO NOT EDIT\n"
        . $source_line
        . "# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens $source_file\n"
        . "#\n"
        . "# This file embeds a TokenGrammar as native Perl data structures.\n"
        . "# Call token_grammar() instead of reading and parsing the .tokens file.\n"
        . "\n"
        . $body
        . "\n1;\n";
}

# ----------------------------------------------------------------------------
# Parser-grammar element rendering
# ----------------------------------------------------------------------------
sub _element_src {
    my ($element, $indent) = @_;
    return 'undef' unless $element;
    my $i = $indent . '    ';
    my $type = $element->{type};

    if ($type eq 'rule_reference') {
        return "{ type => 'rule_reference', name => " . _perl_string($element->{name})
            . ", is_token => " . _perl_bool($element->{is_token}) . " }";
    }
    elsif ($type eq 'literal') {
        return "{ type => 'literal', value => " . _perl_string($element->{value}) . " }";
    }
    elsif ($type eq 'sequence') {
        my $items = join(",\n", map { "$i" . _element_src($_, $i) } @{ $element->{elements} });
        return "{ type => 'sequence', elements => [\n$items,\n$indent] }";
    }
    elsif ($type eq 'alternation') {
        my $items = join(",\n", map { "$i" . _element_src($_, $i) } @{ $element->{choices} });
        return "{ type => 'alternation', choices => [\n$items,\n$indent] }";
    }
    elsif ($type eq 'repetition') {
        return "{ type => 'repetition', element => " . _element_src($element->{element}, $i) . " }";
    }
    elsif ($type eq 'optional') {
        return "{ type => 'optional', element => " . _element_src($element->{element}, $i) . " }";
    }
    elsif ($type eq 'group') {
        return "{ type => 'group', element => " . _element_src($element->{element}, $i) . " }";
    }
    elsif ($type eq 'positive_lookahead') {
        return "{ type => 'positive_lookahead', element => " . _element_src($element->{element}, $i) . " }";
    }
    elsif ($type eq 'negative_lookahead') {
        return "{ type => 'negative_lookahead', element => " . _element_src($element->{element}, $i) . " }";
    }
    elsif ($type eq 'one_or_more') {
        return "{ type => 'one_or_more', element => " . _element_src($element->{element}, $i) . " }";
    }
    elsif ($type eq 'separated_repetition') {
        return "{ type => 'separated_repetition', element => " . _element_src($element->{element}, $i)
            . ", separator => " . _element_src($element->{separator}, $i)
            . ", at_least_one => " . _perl_bool($element->{at_least_one}) . " }";
    }
    else {
        die "CodingAdventures::GrammarTools::Compiler: unknown grammar element type: $type";
    }
}

sub _rule_src {
    my ($rule, $indent) = @_;
    my $i = $indent . '    ';
    my $body_src = _element_src($rule->{body}, $i);
    return "${indent}{\n"
        . "${i}name => " . _perl_string($rule->{name}) . ",\n"
        . "${i}body => $body_src,\n"
        . "${i}line_number => " . $rule->{line_number} . ",\n"
        . "${indent}}";
}

# ----------------------------------------------------------------------------
# compile_parser_grammar($grammar, $source_file) -> Perl source string
# ----------------------------------------------------------------------------
sub compile_parser_grammar {
    my ($grammar, $source_file) = @_;
    $source_file //= '';

    my $rules = $grammar->{rules} // [];
    my $rules_src;
    if (@$rules) {
        my $items = join(",\n", map { _rule_src($_, '        ') } @$rules);
        $rules_src = "[\n$items,\n    ]";
    }
    else {
        $rules_src = '[]';
    }

    my $source_line = $source_file eq '' ? '' : "# Source: $source_file\n";
    my $version = $grammar->{version} // 0;

    return <<"PERL";
# AUTO-GENERATED FILE — DO NOT EDIT
${source_line}# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-grammar $source_file
#
# This file embeds a ParserGrammar as native Perl data structures.
# Call parser_grammar() instead of reading and parsing the .grammar file.

sub parser_grammar {
    return {
        rules => $rules_src,
        version => $version,
    };
}

1;
PERL
}

1;
