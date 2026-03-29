use strict;
use warnings;
use Test2::V0;

use CodingAdventures::DocumentAstSanitizer;

# Short alias for the module namespace
use constant DS => 'CodingAdventures::DocumentAstSanitizer';

# Convenience aliases to the functional API
sub sanitize          { CodingAdventures::DocumentAstSanitizer::sanitize(@_) }
sub sanitize_children { CodingAdventures::DocumentAstSanitizer::sanitize_children(@_) }
sub with_defaults     { CodingAdventures::DocumentAstSanitizer::with_defaults(@_) }

# ---------------------------------------------------------------------------
# 1. Module version
# ---------------------------------------------------------------------------
is(CodingAdventures::DocumentAstSanitizer->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# 2-4. Preset policies exist
# ---------------------------------------------------------------------------
ok(ref $CodingAdventures::DocumentAstSanitizer::STRICT,      '$STRICT is a hashref');
ok(ref $CodingAdventures::DocumentAstSanitizer::RELAXED,     '$RELAXED is a hashref');
ok(ref $CodingAdventures::DocumentAstSanitizer::PASSTHROUGH, '$PASSTHROUGH is a hashref');

# ---------------------------------------------------------------------------
# 5. sanitize() preserves document node on empty doc
# ---------------------------------------------------------------------------
my $empty_doc = {type => 'document', children => []};
my $result = sanitize($empty_doc, {});
is($result->{type}, 'document', 'sanitize returns document node');
is(scalar @{$result->{children}}, 0, 'empty document stays empty');

# ---------------------------------------------------------------------------
# 6. text nodes pass through
# ---------------------------------------------------------------------------
my $doc = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{type => 'text', value => 'hello'}],
    }],
};
my $out = sanitize($doc, {});
is($out->{children}[0]{type}, 'paragraph', 'paragraph preserved');
is($out->{children}[0]{children}[0]{type}, 'text', 'text node preserved');
is($out->{children}[0]{children}[0]{value}, 'hello', 'text value preserved');

# ---------------------------------------------------------------------------
# 7. raw_block dropped by STRICT
# ---------------------------------------------------------------------------
my $doc2 = {
    type     => 'document',
    children => [
        {type => 'raw_block', format => 'html', value => '<script>alert(1)</script>'},
        {type => 'paragraph', children => [{type => 'text', value => 'safe'}]},
    ],
};
my $out2 = sanitize($doc2, $CodingAdventures::DocumentAstSanitizer::STRICT);
is(scalar @{$out2->{children}}, 1, 'raw_block dropped by STRICT');
is($out2->{children}[0]{type}, 'paragraph', 'paragraph remains after raw_block drop');

# ---------------------------------------------------------------------------
# 8. raw_block passes through with PASSTHROUGH
# ---------------------------------------------------------------------------
my $out3 = sanitize($doc2, $CodingAdventures::DocumentAstSanitizer::PASSTHROUGH);
is(scalar @{$out3->{children}}, 2, 'raw_block kept by PASSTHROUGH');

# ---------------------------------------------------------------------------
# 9. javascript: link destination blanked
# ---------------------------------------------------------------------------
my $doc4 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{
            type        => 'link',
            destination => 'javascript:alert(1)',
            title       => undef,
            children    => [{type => 'text', value => 'click'}],
        }],
    }],
};
my $out4 = sanitize($doc4, $CodingAdventures::DocumentAstSanitizer::STRICT);
my $link  = $out4->{children}[0]{children}[0];
is($link->{type}, 'link', 'link node preserved');
is($link->{destination}, '', 'javascript: destination blanked');

# ---------------------------------------------------------------------------
# 10. https: link allowed by STRICT
# ---------------------------------------------------------------------------
my $doc5 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{
            type        => 'link',
            destination => 'https://example.com',
            title       => undef,
            children    => [{type => 'text', value => 'visit'}],
        }],
    }],
};
my $out5 = sanitize($doc5, $CodingAdventures::DocumentAstSanitizer::STRICT);
is($out5->{children}[0]{children}[0]{destination}, 'https://example.com', 'https link preserved');

# ---------------------------------------------------------------------------
# 11. dropLinks promotes children
# ---------------------------------------------------------------------------
my $policy11 = {dropLinks => 1, allowedUrlSchemes => 0};
my $out11 = sanitize($doc5, $policy11);
is($out11->{children}[0]{children}[0]{type}, 'text', 'link dropped; text child promoted');
is($out11->{children}[0]{children}[0]{value}, 'visit', 'promoted text value correct');

# ---------------------------------------------------------------------------
# 12. transformImageToText replaces image with alt text
# ---------------------------------------------------------------------------
my $doc12 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{
            type        => 'image',
            destination => 'https://example.com/img.png',
            title       => undef,
            alt         => 'My Photo',
        }],
    }],
};
my $out12 = sanitize($doc12, $CodingAdventures::DocumentAstSanitizer::STRICT);
is($out12->{children}[0]{children}[0]{type}, 'text', 'image transformed to text');
is($out12->{children}[0]{children}[0]{value}, 'My Photo', 'alt text used as text value');

# ---------------------------------------------------------------------------
# 13. dropImages removes images entirely (empty paragraph pruned)
# ---------------------------------------------------------------------------
my $policy13 = {dropImages => 1, allowedUrlSchemes => 0};
my $out13 = sanitize($doc12, $policy13);
is(scalar @{$out13->{children}}, 0, 'paragraph dropped when image removed');

# ---------------------------------------------------------------------------
# 14. maxHeadingLevel clamps heading levels
# ---------------------------------------------------------------------------
my $doc14 = {
    type     => 'document',
    children => [{
        type     => 'heading',
        level    => 1,
        children => [{type => 'text', value => 'Title'}],
    }],
};
my $out14 = sanitize($doc14, $CodingAdventures::DocumentAstSanitizer::STRICT);
is($out14->{children}[0]{level}, 2, 'h1 clamped to minHeadingLevel=2 by STRICT');

# ---------------------------------------------------------------------------
# 15. maxHeadingLevel = "drop" removes all headings
# ---------------------------------------------------------------------------
my $policy15 = {maxHeadingLevel => 'drop'};
my $out15 = sanitize($doc14, $policy15);
is(scalar @{$out15->{children}}, 0, 'all headings dropped when maxHeadingLevel="drop"');

# ---------------------------------------------------------------------------
# 16. code_block passes through by default
# ---------------------------------------------------------------------------
my $doc16 = {
    type     => 'document',
    children => [{type => 'code_block', language => 'perl', value => 'say "hi"'}],
};
my $out16 = sanitize($doc16, {});
is($out16->{children}[0]{type}, 'code_block', 'code_block preserved');

# ---------------------------------------------------------------------------
# 17. dropCodeBlocks removes code blocks
# ---------------------------------------------------------------------------
my $out17 = sanitize($doc16, {dropCodeBlocks => 1});
is(scalar @{$out17->{children}}, 0, 'code_block dropped when dropCodeBlocks=1');

# ---------------------------------------------------------------------------
# 18. transformCodeSpanToText converts code spans
# ---------------------------------------------------------------------------
my $doc18 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{type => 'code_span', value => 'my $x = 1'}],
    }],
};
my $out18 = sanitize($doc18, {transformCodeSpanToText => 1});
is($out18->{children}[0]{children}[0]{type}, 'text', 'code_span transformed to text');
is($out18->{children}[0]{children}[0]{value}, 'my $x = 1', 'code_span value preserved');

# ---------------------------------------------------------------------------
# 19. strong and emphasis nodes recurse
# ---------------------------------------------------------------------------
my $doc19 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{
            type     => 'strong',
            children => [{type => 'text', value => 'bold'}],
        }],
    }],
};
my $out19 = sanitize($doc19, {});
is($out19->{children}[0]{children}[0]{type}, 'strong', 'strong node preserved');
is($out19->{children}[0]{children}[0]{children}[0]{value}, 'bold', 'text inside strong preserved');

# ---------------------------------------------------------------------------
# 20. thematic_break always kept
# ---------------------------------------------------------------------------
my $doc20 = {
    type     => 'document',
    children => [{type => 'thematic_break'}],
};
my $out20 = sanitize($doc20, $CodingAdventures::DocumentAstSanitizer::STRICT);
is($out20->{children}[0]{type}, 'thematic_break', 'thematic_break always kept');

# ---------------------------------------------------------------------------
# 21. dropBlockquotes removes blockquotes
# ---------------------------------------------------------------------------
my $doc21 = {
    type     => 'document',
    children => [{
        type     => 'blockquote',
        children => [{
            type     => 'paragraph',
            children => [{type => 'text', value => 'quoted'}],
        }],
    }],
};
my $out21 = sanitize($doc21, {dropBlockquotes => 1});
is(scalar @{$out21->{children}}, 0, 'blockquote dropped');

# ---------------------------------------------------------------------------
# 22. lists recurse through items
# ---------------------------------------------------------------------------
my $doc22 = {
    type     => 'document',
    children => [{
        type     => 'list',
        ordered  => 0,
        tight    => 1,
        start    => 1,
        children => [{
            type     => 'list_item',
            children => [{
                type     => 'paragraph',
                children => [{type => 'text', value => 'item 1'}],
            }],
        }],
    }],
};
my $out22 = sanitize($doc22, {});
is($out22->{children}[0]{type}, 'list', 'list preserved');
is($out22->{children}[0]{children}[0]{type}, 'list_item', 'list_item preserved');

# ---------------------------------------------------------------------------
# 23. with_defaults merges over PASSTHROUGH
# ---------------------------------------------------------------------------
my $custom = with_defaults({dropLinks => 1});
is($custom->{dropLinks}, 1, 'with_defaults: override applied');
is($custom->{dropCodeBlocks}, 0, 'with_defaults: passthrough default preserved');

# ---------------------------------------------------------------------------
# 24. Unknown inline nodes are dropped
# ---------------------------------------------------------------------------
my $doc24 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [
            {type => 'alien_node', value => 'dangerous'},
            {type => 'text', value => 'safe'},
        ],
    }],
};
my $out24 = sanitize($doc24, {});
is(scalar @{$out24->{children}[0]{children}}, 1, 'unknown inline node dropped');
is($out24->{children}[0]{children}[0]{value}, 'safe', 'known node kept');

# ---------------------------------------------------------------------------
# 25. hard_break and soft_break pass through
# ---------------------------------------------------------------------------
my $doc25 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [
            {type => 'text', value => 'line 1'},
            {type => 'hard_break'},
            {type => 'text', value => 'line 2'},
            {type => 'soft_break'},
            {type => 'text', value => 'line 3'},
        ],
    }],
};
my $out25 = sanitize($doc25, {});
is(scalar @{$out25->{children}[0]{children}}, 5, 'hard_break and soft_break preserved');
is($out25->{children}[0]{children}[1]{type}, 'hard_break', 'hard_break type correct');
is($out25->{children}[0]{children}[3]{type}, 'soft_break', 'soft_break type correct');

# ---------------------------------------------------------------------------
# 26. Image with javascript: destination is blanked
# ---------------------------------------------------------------------------
my $doc26 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{
            type        => 'image',
            destination => 'javascript:alert(1)',
            title       => undef,
            alt         => 'evil',
        }],
    }],
};
my $policy26 = {
    dropImages           => 0,
    transformImageToText => 0,
    allowedUrlSchemes    => [qw(http https)],
};
my $out26 = sanitize($doc26, $policy26);
is($out26->{children}[0]{children}[0]{type}, 'image', 'image kept when not dropping images');
is($out26->{children}[0]{children}[0]{destination}, '', 'javascript: image destination blanked');

# ---------------------------------------------------------------------------
# 27. sanitize_children() convenience function
# ---------------------------------------------------------------------------
my @children = (
    {type => 'paragraph', children => [{type => 'text', value => 'test'}]},
    {type => 'raw_block', format => 'html', value => '<b>hi</b>'},
);
my $sc = sanitize_children(\@children, $CodingAdventures::DocumentAstSanitizer::STRICT);
is(scalar @$sc, 1, 'sanitize_children drops raw_block with STRICT');
is($sc->[0]{type}, 'paragraph', 'paragraph survives sanitize_children');

# ---------------------------------------------------------------------------
# 28. relative URL always passes through
# ---------------------------------------------------------------------------
my $doc28 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{
            type        => 'link',
            destination => '/about',
            title       => undef,
            children    => [{type => 'text', value => 'About'}],
        }],
    }],
};
my $out28 = sanitize($doc28, $CodingAdventures::DocumentAstSanitizer::STRICT);
is($out28->{children}[0]{children}[0]{destination}, '/about', 'relative URL passes through');

# ---------------------------------------------------------------------------
# 29. autolink with disallowed scheme dropped
# ---------------------------------------------------------------------------
my $doc29 = {
    type     => 'document',
    children => [{
        type     => 'paragraph',
        children => [{
            type        => 'autolink',
            destination => 'javascript:void(0)',
            is_email    => 0,
        }],
    }],
};
my $out29 = sanitize($doc29, $CodingAdventures::DocumentAstSanitizer::STRICT);
is(scalar @{$out29->{children}[0]{children}}, 0, 'autolink with bad scheme dropped');

done_testing;
