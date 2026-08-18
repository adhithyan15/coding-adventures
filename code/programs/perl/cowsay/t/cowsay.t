use strict;
use warnings;
use utf8;

use FindBin qw($Bin);
use lib "$Bin/../lib";
use lib "$Bin/../../../../packages/perl/cli-builder/lib";
use lib "$Bin/../../../../packages/perl/paint-instructions/lib";
use lib "$Bin/../../../../packages/perl/paint-vm-ascii/lib";

use Test2::V0;
use File::Spec;
use File::Temp qw(tempdir);

use CodingAdventures::Cowsay;
use CodingAdventures::CliBuilder;

# --- wrap_text ---------------------------------------------------------------

subtest 'wrap_text' => sub {
    is(CodingAdventures::Cowsay::wrap_text('hello', 40), ['hello'], 'short text is not wrapped');

    is(
        CodingAdventures::Cowsay::wrap_text('the quick brown fox jumps over', 10),
        [ 'the quick', 'brown fox', 'jumps over' ],
        'long text wraps at word boundaries',
    );

    is(CodingAdventures::Cowsay::wrap_text('', 40), [''], 'empty text returns an empty line');

    is(
        CodingAdventures::Cowsay::wrap_text('supercalifragilisticexpialidocious', 5),
        ['supercalifragilisticexpialidocious'],
        'a single word longer than the width stays whole',
    );

    is(CodingAdventures::Cowsay::wrap_text('     ', 3), [''], 'whitespace-only text returns an empty line');
};

# --- format_bubble -------------------------------------------------------------

subtest 'format_bubble' => sub {
    is(CodingAdventures::Cowsay::format_bubble([], 0), '', 'empty lines returns empty string');

    is(
        CodingAdventures::Cowsay::format_bubble(['hi'], 0),
        " ____\n< hi >\n ----",
        'single-line speech bubble',
    );

    is(
        CodingAdventures::Cowsay::format_bubble(['hi'], 1),
        " ____\n( hi )\n ----",
        'single-line thought bubble',
    );

    is(
        CodingAdventures::Cowsay::format_bubble([ 'one', 'two', 'three' ], 0),
        " _______\n/ one   \\\n| two   |\n\\ three /\n -------",
        'multi-line speech bubble uses slash/pipe/backslash borders',
    );

    is(
        CodingAdventures::Cowsay::format_bubble([ 'one', 'two' ], 1),
        " _____\n( one )\n( two )\n -----",
        'multi-line thought bubble uses parens on every line',
    );
};

# --- normalize_two_chars -------------------------------------------------------

subtest 'normalize_two_chars' => sub {
    is(CodingAdventures::Cowsay::normalize_two_chars('o'),   'o ', 'pads a 1-char value');
    is(CodingAdventures::Cowsay::normalize_two_chars(''),    '  ', 'pads an empty value');
    is(CodingAdventures::Cowsay::normalize_two_chars('oo'),  'oo', 'a 2-char value is unchanged');
    is(CodingAdventures::Cowsay::normalize_two_chars('ooo'), 'oo', 'truncates a longer value');
};

# --- resolve_eyes_and_tongue ---------------------------------------------------

subtest 'resolve_eyes_and_tongue' => sub {
    is([ CodingAdventures::Cowsay::resolve_eyes_and_tongue('oo', '  ', []) ], [ 'oo', '  ' ], 'no active modes keeps base values');

    my %expected = (
        borg     => [ '==', '  ' ],
        dead     => [ 'XX', 'U ' ],
        greedy   => [ '$$', '  ' ],
        paranoid => [ '@@', '  ' ],
        stoned   => [ 'xx', 'U ' ],
        tired    => [ '--', '  ' ],
        wired    => [ 'OO', '  ' ],
        youthful => [ '..', '  ' ],
    );
    for my $mode (sort keys %expected) {
        is(
            [ CodingAdventures::Cowsay::resolve_eyes_and_tongue('oo', '  ', [$mode]) ],
            $expected{$mode},
            "mode '$mode' overrides eyes and sometimes tongue",
        );
    }

    is(
        [ CodingAdventures::Cowsay::resolve_eyes_and_tongue('oo', '  ', ['not-a-real-mode']) ],
        [ 'oo', '  ' ],
        'an unknown mode is ignored',
    );
};

# --- load_cow ------------------------------------------------------------------

subtest 'load_cow' => sub {
    my $temp_dir = tempdir(CLEANUP => 1);

    subtest 'loads the body between heredoc markers' => sub {
        open(my $fh, '>:encoding(UTF-8)', File::Spec->catfile($temp_dir, 'default.cow'));
        print $fh "\$the_cow = <<EOC;\n  \$thoughts   ^__^\n   (\$eyes)\nEOC\n";
        close $fh;

        is(
            CodingAdventures::Cowsay::load_cow('default', $temp_dir),
            "  \$thoughts   ^__^\n   (\$eyes)\n",
        );
    };

    subtest 'falls back to default.cow when the named cow is missing' => sub {
        open(my $fh, '>:encoding(UTF-8)', File::Spec->catfile($temp_dir, 'default.cow'));
        print $fh "\$the_cow = <<EOC;\nfallback\nEOC\n";
        close $fh;

        is(CodingAdventures::Cowsay::load_cow('does-not-exist', $temp_dir), "fallback\n");
    };

    subtest 'falls back to default.cow instead of escaping via traversal' => sub {
        my $outside_dir = tempdir(CLEANUP => 1);
        open(my $fh, '>:encoding(UTF-8)', File::Spec->catfile($outside_dir, 'secret.cow'));
        print $fh "\$the_cow = <<EOC;\nSECRET\nEOC\n";
        close $fh;
        open(my $fh2, '>:encoding(UTF-8)', File::Spec->catfile($outside_dir, 'outside.cow'));
        print $fh2 "\$the_cow = <<EOC;\nSECRET\nEOC\n";
        close $fh2;

        for my $malicious ('../../../../../../etc/passwd', '..\\..\\..\\secret', '../outside') {
            is(
                CodingAdventures::Cowsay::load_cow($malicious, $temp_dir),
                "fallback\n",
                "traversal attempt '$malicious' falls back to default.cow",
            );
        }
    };

    subtest 'falls back to default.cow instead of following a rooted path override' => sub {
        my $outside_dir = tempdir(CLEANUP => 1);
        my $rooted_target = File::Spec->catfile($outside_dir, 'win');
        open(my $fh, '>:encoding(UTF-8)', "$rooted_target.cow");
        print $fh "\$the_cow = <<EOC;\nSECRET\nEOC\n";
        close $fh;

        is(CodingAdventures::Cowsay::load_cow($rooted_target, $temp_dir), "fallback\n");
    };
};

# --- compose_content -------------------------------------------------------------

subtest 'compose_content' => sub {
    my $temp_dir = tempdir(CLEANUP => 1);
    open(my $fh, '>:encoding(UTF-8)', File::Spec->catfile($temp_dir, 'default.cow'));
    print $fh "\$the_cow = <<EOC;\n\$thoughts \$eyes \$tongue\nEOC\n";
    close $fh;

    my $base_invocation = {
        message      => 'hi',
        eyes         => 'oo',
        tongue       => '  ',
        active_modes => [],
        nowrap       => 0,
        width        => 40,
        think        => 0,
        cowfile      => 'default',
    };

    is(
        CodingAdventures::Cowsay::compose_content($base_invocation, $temp_dir),
        " ____\n< hi >\n ----\n\\ oo   \n",
        'composes bubble and cow with substitutions',
    );

    my $think_invocation = { %$base_invocation, think => 1 };
    is(
        CodingAdventures::Cowsay::compose_content($think_invocation, $temp_dir),
        " ____\n( hi )\n ----\no oo   \n",
        'think mode uses o for thoughts and a paren bubble',
    );

    my $dead_invocation = { %$base_invocation, active_modes => ['dead'] };
    is(
        CodingAdventures::Cowsay::compose_content($dead_invocation, $temp_dir),
        " ____\n< hi >\n ----\n\\ XX U \n",
        'a mode flag overrides eyes (and tongue) in the cow template',
    );
};

# --- build_scene -------------------------------------------------------------

subtest 'build_scene' => sub {
    my $scene = CodingAdventures::Cowsay::build_scene("hi\n\nyo");
    my @glyph_runs = grep { $_->{kind} eq 'glyph_run' } @{ $scene->{instructions} };
    is(scalar @glyph_runs, 2, 'one glyph_run per non-blank line');

    is($glyph_runs[0]{glyphs}, [
        { glyph_id => ord('h'), x => 0, y => 0 },
        { glyph_id => ord('i'), x => CodingAdventures::Cowsay::SCALE_X, y => 0 },
    ]);

    is($glyph_runs[1]{glyphs}, [
        { glyph_id => ord('y'), x => 0, y => 2 * CodingAdventures::Cowsay::SCALE_Y },
        { glyph_id => ord('o'), x => CodingAdventures::Cowsay::SCALE_X, y => 2 * CodingAdventures::Cowsay::SCALE_Y },
    ]);

    my $spaces_scene = CodingAdventures::Cowsay::build_scene('a b');
    my @spaces_runs = grep { $_->{kind} eq 'glyph_run' } @{ $spaces_scene->{instructions} };
    is(scalar @spaces_runs, 1, 'spaces are skipped, not placed');
    is(scalar @{ $spaces_runs[0]{glyphs} }, 2);

    my $dims_scene = CodingAdventures::Cowsay::build_scene("abc\nde");
    is($dims_scene->{width},  3 * CodingAdventures::Cowsay::SCALE_X);
    is($dims_scene->{height}, 2 * CodingAdventures::Cowsay::SCALE_Y);
};

# --- render round trip -----------------------------------------------------------

subtest 'render round-trips through paint-vm-ascii' => sub {
    for my $content ('hi', "hello\nworld", " ____\n< hi >\n ----\n\\   ^__^\n") {
        my $scene = CodingAdventures::Cowsay::build_scene($content);
        my $output = CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 });

        my @expected_lines = map { my $l = $_; $l =~ s/\s+$//; $l } split(/\n/, $content, -1);
        my $expected = join("\n", @expected_lines);
        $expected =~ s/[\s\n]+$//;

        is($output, $expected, "round-trips '$content'");
    }
};

# --- CLI glue -----------------------------------------------------------------

subtest 'is_list_requested' => sub {
    ok(CodingAdventures::Cowsay::is_list_requested({ list => 1 }));
    ok(!CodingAdventures::Cowsay::is_list_requested({}));
    ok(!CodingAdventures::Cowsay::is_list_requested({ list => 0 }));
};

subtest 'resolve_message_from_arguments' => sub {
    is(CodingAdventures::Cowsay::resolve_message_from_arguments({ message => [ 'hello', 'there' ] }), 'hello there');
    is(CodingAdventures::Cowsay::resolve_message_from_arguments({}), undef);
    is(CodingAdventures::Cowsay::resolve_message_from_arguments({ message => [] }), undef);
};

subtest 'build_invocation' => sub {
    my $invocation = CodingAdventures::Cowsay::build_invocation('hi', {});
    is($invocation->{message}, 'hi');
    is($invocation->{eyes}, 'oo');
    is($invocation->{tongue}, '  ');
    is($invocation->{cowfile}, 'default');
    ok(!$invocation->{nowrap});
    ok(!$invocation->{think});
    is($invocation->{width}, 40);
    is($invocation->{active_modes}, []);

    my $explicit = CodingAdventures::Cowsay::build_invocation('hi', {
        eyes => '^^', tongue => 'vv', cowfile => 'dragon',
        nowrap => 1, think => 1, width => 20, borg => 1,
    });
    is($explicit->{eyes}, '^^');
    is($explicit->{tongue}, 'vv');
    is($explicit->{cowfile}, 'dragon');
    ok($explicit->{nowrap});
    ok($explicit->{think});
    is($explicit->{width}, 20);
    is($explicit->{active_modes}, ['borg']);

    is(CodingAdventures::Cowsay::build_invocation('hi', { width => 99_999_999_999 })->{width}, 2_147_483_647);
    is(CodingAdventures::Cowsay::build_invocation('hi', { width => -5 })->{width}, 1);
};

subtest 'list_cow_files' => sub {
    my $temp_dir = tempdir(CLEANUP => 1);
    for my $name (qw(tux default dragon)) {
        open(my $fh, '>', File::Spec->catfile($temp_dir, "$name.cow"));
        close $fh;
    }
    is(CodingAdventures::Cowsay::list_cow_files($temp_dir), [ 'default', 'dragon', 'tux' ]);
};

# --- Parser argv convention regression -----------------------------------------
#
# Unlike the C#/F# CliBuilder ports, this one does NOT expect a leading
# program-name placeholder in argv -- Parser->parse iterates the whole array
# from index 0. This test documents and locks in that (different!) contract
# so a future change to the shared spec doesn't silently break either
# assumption. See the "C#" lessons.md entry for the opposite convention.

subtest 'CliBuilder does not expect a leading program-name placeholder' => sub {
    my $repo_root = CodingAdventures::Cowsay::find_repo_root($Bin);
    my $spec_path = File::Spec->catfile($repo_root, 'code', 'specs', 'cowsay.json');

    open(my $fh, '<:encoding(UTF-8)', $spec_path) or die "cannot read $spec_path: $!";
    my $spec_json = do { local $/; <$fh> };
    close $fh;

    require JSON::PP;
    my $spec_raw = JSON::PP::decode_json($spec_json);

    my $single = CodingAdventures::CliBuilder->parse_hashref($spec_raw, ['hello']);
    is($single->{type}, 'result');
    is(CodingAdventures::Cowsay::resolve_message_from_arguments($single->{arguments}), 'hello');

    my $multi = CodingAdventures::CliBuilder->parse_hashref($spec_raw, [ 'hello', 'world' ]);
    is($multi->{type}, 'result');
    is(CodingAdventures::Cowsay::resolve_message_from_arguments($multi->{arguments}), 'hello world');
};

# --- End-to-end golden tests -----------------------------------------------------

subtest 'end-to-end golden output' => sub {
    my $repo_root = CodingAdventures::Cowsay::find_repo_root($Bin);
    my $cows_dir  = File::Spec->catfile($repo_root, 'code', 'specs', 'cows');

    ok(-d $cows_dir, "cows dir resolved at $cows_dir");
    ok(-e File::Spec->catfile($cows_dir, 'default.cow'));

    my $hello_invocation = {
        message => 'Hello, World!', eyes => 'oo', tongue => '  ', active_modes => [],
        nowrap => 0, width => 40, think => 0, cowfile => 'default',
    };
    is(
        CodingAdventures::Cowsay::render($hello_invocation, $cows_dir),
        join("\n",
            ' _______________',
            '< Hello, World! >',
            ' ---------------',
            '        \   ^__^',
            '         \  (oo)\_______',
            '            (__)\       )\/\\',
            '                ||----w |',
            '                ||     ||'),
        'default cow speaking Hello, World!',
    );

    my $borg_invocation = {
        message => 'beep', eyes => 'oo', tongue => '  ', active_modes => ['borg'],
        nowrap => 0, width => 40, think => 1, cowfile => 'default',
    };
    is(
        CodingAdventures::Cowsay::render($borg_invocation, $cows_dir),
        join("\n",
            ' ______',
            '( beep )',
            ' ------',
            '        o   ^__^',
            '         o  (==)\_______',
            '            (__)\       )\/\\',
            '                ||----w |',
            '                ||     ||'),
        'borg mode thinking with the default cow',
    );
};

done_testing;
