#!/usr/bin/env perl

use strict;
use warnings;
use utf8;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use JSON::PP ();
use Test2::V0;

use CodingAdventures::BuildTool::ToolchainDetection;

my $CASES = "$Bin/../../../../specs/fixtures/build-tool-v1/cases";
my @EXPECTED_FIXTURES = qw(
    toolchain-detection-affected-only.json
    toolchain-detection-crlf-grammar.json
    toolchain-detection-declarations.json
    toolchain-detection-empty.json
    toolchain-detection-force-full.json
    toolchain-detection-null-all.json
    toolchain-detection-platform-darwin.json
    toolchain-detection-platform-linux.json
    toolchain-detection-platform-windows.json
    toolchain-detection-shared.json
    toolchain-detection-unsupported.json
);

sub read_json {
    my ($path) = @_;
    open(my $fh, '<:raw', $path) or die "Cannot read $path: $!";
    local $/;
    my $payload = <$fh>;
    close($fh);
    return JSON::PP->new->utf8->decode($payload);
}

subtest 'independently consumes every neutral toolchain-detection fixture' => sub {
    opendir(my $dh, $CASES) or die "Cannot enumerate $CASES: $!";
    my @fixtures = sort grep {
        /^toolchain-detection-.+\.json\z/
    } readdir($dh);
    closedir($dh);

    is(\@fixtures, \@EXPECTED_FIXTURES, 'discovers the exact 11-case corpus');

    for my $filename (@fixtures) {
        my $fixture = read_json("$CASES/$filename");
        my $options = $fixture->{input}{options};
        my $expected = $fixture->{expected};
        my $actual = CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
            $options->{platform},
            $options->{force_full} ? 1 : 0,
            $options->{packages},
            $options->{scheduled_packages},
            $options->{forced_toolchains},
        );

        subtest $fixture->{id} => sub {
            is($actual->{outcome}, $expected->{outcome}, 'outcome');
            is(
                $actual->{toolchains},
                $expected->{result}{toolchains} || {},
                'complete canonical flags',
            );
            is($actual->{diagnostics}, $expected->{diagnostics}, 'diagnostics');
        };
    }
};

subtest 'rejects byte, line, and aggregate snapshot overruns' => sub {
    my $per_file = dies {
        CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
            'linux', 0,
            [{ name => 'rust/app', language => 'rust', build_files => { BUILD => 'x' x 65_537 } }],
            undef, [],
        );
    };
    like($per_file, qr/per-file resource ceiling/, 'rejects oversized byte string');

    my $unicode_bytes = dies {
        CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
            'linux', 0,
            [{ name => 'rust/app', language => 'rust', build_files => { BUILD => "é" x 32_769 } }],
            undef, [],
        );
    };
    like($unicode_bytes, qr/per-file resource ceiling/, 'counts UTF-8 bytes, not characters');

    my $too_many_lines = dies {
        CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
            'linux', 0,
            [{ name => 'rust/app', language => 'rust', build_files => { BUILD => "\n" x 4_096 } }],
            undef, [],
        );
    };
    like($too_many_lines, qr/per-file resource ceiling/, 'rejects 4,097 logical lines');

    my %build_files = map { ("BUILD_$_" => 'x' x 65_536) } 0 .. 16;
    my $aggregate = dies {
        CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
            'linux', 0,
            [{ name => 'rust/app', language => 'rust', build_files => \%build_files }],
            undef, [],
        );
    };
    like($aggregate, qr/aggregate resource ceiling/, 'rejects oversized aggregate');

    my %exact_aggregate = map { ("BUILD_$_" => 'x' x 65_536) } 0 .. 15;
    ok(
        lives {
            CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
                'linux', 0,
                [{ name => 'rust/app', language => 'rust', build_files => \%exact_aggregate }],
                undef, [],
            );
        },
        'accepts the exact 1 MiB aggregate ceiling',
    );
    ok(
        lives {
            CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
                'linux', 0,
                [{ name => 'rust/app', language => 'rust', build_files => { BUILD => "\n" x 4_095 } }],
                undef, [],
            );
        },
        'accepts the exact 4,096-line ceiling',
    );
};

subtest 'keeps declaration grammar byte-exact across CRLF and lone CR' => sub {
    is(
        CodingAdventures::BuildTool::ToolchainDetection::parse_extra_toolchains(
            "  # needs-toolchain: python  \r\n\t# needs-toolchain:\tjava\t\r\n"
        ),
        ['python', 'java'],
        'accepts exact CRLF declarations',
    );
    is(
        CodingAdventures::BuildTool::ToolchainDetection::parse_extra_toolchains(
            "# needs-toolchain: python\r"
        ),
        [],
        'keeps final lone CR as content',
    );
    is(
        CodingAdventures::BuildTool::ToolchainDetection::parse_extra_toolchains(
            "# needs-toolchain: lua\r  "
        ),
        [],
        'keeps CR before trailing spaces as content',
    );
    is(
        CodingAdventures::BuildTool::ToolchainDetection::parse_extra_toolchains(
            "# needs-toolchain: swift\r\r\n"
        ),
        [],
        'strips only the CR paired with LF',
    );
};

subtest 'preserves empty-front precedence and caller-owned inputs' => sub {
    my $packages = [{
        name => 'rust/app',
        language => 'rust',
        build_files => {
            BUILD => "# needs-toolchain: java\n",
            BUILD_windows => '',
        },
    }];
    my $before = JSON::PP->new->canonical->encode($packages);
    my $actual = CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
        'windows', 0, $packages, undef, ['kotlin'],
    );

    ok($actual->{toolchains}{rust}, 'enables the selected package language');
    ok($actual->{toolchains}{kotlin}, 'unions forced toolchains');
    ok(!$actual->{toolchains}{java}, 'empty Windows front beats generic fallback');
    is(JSON::PP->new->canonical->encode($packages), $before, 'does not mutate caller input');
};

subtest 'returns a fresh canonical registry array' => sub {
    my $first = CodingAdventures::BuildTool::ToolchainDetection::canonical_toolchains();
    $first->[0] = 'changed';
    is(
        CodingAdventures::BuildTool::ToolchainDetection::canonical_toolchains()->[0],
        'cpp',
        'registry cannot be mutated through a returned reference',
    );

    my $packages = [{ name => 'rust/app', language => 'rust', build_files => { BUILD => '' } }];
    my $first_result = CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
        'linux', 0, $packages, undef, [],
    );
    $first_result->{toolchains}{cpp} = JSON::PP::true;
    my $second_result = CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
        'linux', 0, $packages, undef, [],
    );
    ok(!$second_result->{toolchains}{cpp}, 'result maps are fresh for every evaluation');
};

subtest 'keeps unsupported package and forced diagnostics stable' => sub {
    my $unsupported_package = CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
        'linux', 1,
        [{ name => 'zig/app', language => 'zig', build_files => { BUILD => '' } }],
        undef, [],
    );
    is(
        $unsupported_package->{diagnostics},
        [{ code => 'TOOLCHAIN_UNSUPPORTED', severity => 'error', package => 'zig/app' }],
        'force-full still validates selected package languages',
    );

    my $unsupported_forced = CodingAdventures::BuildTool::ToolchainDetection::evaluate_snapshot(
        'linux', 0,
        [{ name => 'rust/app', language => 'rust', build_files => { BUILD => '' } }],
        [], ['zig'],
    );
    is(
        $unsupported_forced->{diagnostics},
        [{ code => 'TOOLCHAIN_UNSUPPORTED', severity => 'error' }],
        'unsupported forced value omits the package field',
    );
};

done_testing;
