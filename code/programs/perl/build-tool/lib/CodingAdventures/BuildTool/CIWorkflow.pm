package CodingAdventures::BuildTool::CIWorkflow;

use strict;
use warnings;

our $CI_WORKFLOW_PATH = '.github/workflows/ci.yml';

my %TOOLCHAIN_MARKERS = (
    python => [
        'needs_python', 'setup-python', 'python-version', 'setup-uv',
        'python --version', 'uv --version', 'pytest',
        'set up python', 'install uv',
    ],
    ruby => [
        'needs_ruby', 'setup-ruby', 'ruby-version', 'bundler',
        'gem install bundler', 'ruby --version', 'bundle --version',
        'set up ruby', 'install bundler',
    ],
    go => [
        'needs_go', 'setup-go', 'go-version', 'go version', 'set up go',
    ],
    typescript => [
        'needs_typescript', 'setup-node', 'node-version', 'npm install -g jest',
        'node --version', 'npm --version', 'set up node',
    ],
    rust => [
        'needs_rust', 'rust-toolchain', 'cargo', 'rustc', 'tarpaulin',
        'wasm32-unknown-unknown', 'set up rust', 'install cargo-tarpaulin',
    ],
    elixir => [
        'needs_elixir', 'setup-beam', 'elixir-version', 'otp-version',
        'elixir --version', 'mix --version', 'set up elixir',
    ],
    lua => [
        'needs_lua', 'gh-actions-lua', 'gh-actions-luarocks', 'luarocks',
        'lua -v', 'msvc', 'set up lua', 'set up luarocks',
    ],
    perl => [
        'needs_perl', 'cpanm', 'perl --version', 'install cpanm',
    ],
    haskell => [
        'needs_haskell', 'haskell-actions/setup', 'ghc-version', 'cabal-version',
        'ghc --version', 'cabal --version', 'set up haskell',
    ],
    java => [
        'needs_java', 'setup-java', 'java-version', 'java --version',
        'temurin', 'set up jdk', 'set up gradle', 'setup-gradle',
        'disable long-lived gradle services',
        'gradle_opts', 'org.gradle.daemon', 'org.gradle.vfs.watch',
    ],
    kotlin => [
        'needs_kotlin', 'setup-java', 'java-version',
        'temurin', 'set up jdk', 'set up gradle', 'setup-gradle',
        'disable long-lived gradle services',
        'gradle_opts', 'org.gradle.daemon', 'org.gradle.vfs.watch',
    ],
    dotnet => [
        'needs_dotnet', 'setup-dotnet', 'dotnet-version', 'dotnet --version',
        'set up .net',
    ],
);

my @UNSAFE_MARKERS = (
    './build-tool',
    'build-tool.exe',
    '-detect-languages',
    '-emit-plan',
    '-force',
    '-plan-file',
    '-validate-build-files',
    'actions/checkout',
    'build-plan',
    'cancel-in-progress:',
    'concurrency:',
    'diff-base',
    'download-artifact',
    'event_name',
    'fetch-depth',
    'git fetch origin main',
    'git_ref',
    'is_main',
    'matrix:',
    'permissions:',
    'pr_base_ref',
    'pull_request:',
    'push:',
    'runs-on:',
    'strategy:',
    'upload-artifact',
);

sub ci_workflow_path {
    return $CI_WORKFLOW_PATH;
}

sub analyze_changes {
    my ($root, $diff_base) = @_;
    return analyze_patch(_file_diff($root, $diff_base, $CI_WORKFLOW_PATH));
}

sub analyze_patch {
    my ($patch) = @_;
    my %toolchains;
    my @hunk;

    for my $line (split /\n/, $patch // q{}) {
        if ($line =~ /^@@/) {
            my ($hunk_toolchains, $unsafe) = _classify_hunk(\@hunk);
            return { toolchains => {}, requires_full_rebuild => 1 } if $unsafe;
            %toolchains = (%toolchains, %{$hunk_toolchains});
            @hunk = ();
            next;
        }

        next if $line =~ /^(?:diff --git |index |--- |\+\+\+ )/;
        push @hunk, $line;
    }

    my ($hunk_toolchains, $unsafe) = _classify_hunk(\@hunk);
    return { toolchains => {}, requires_full_rebuild => 1 } if $unsafe;
    %toolchains = (%toolchains, %{$hunk_toolchains});

    return {
        toolchains => \%toolchains,
        requires_full_rebuild => 0,
    };
}

sub sorted_toolchains {
    my ($toolchains) = @_;
    return sort keys %{ $toolchains || {} };
}

sub _classify_hunk {
    my ($lines) = @_;
    my %hunk_toolchains;
    my %changed_toolchains;
    my @changed_lines;

    for my $line (@{$lines || []}) {
        next if $line eq q{} || !_is_diff_line($line);

        my $content = substr($line, 1);
        $content =~ s/^\s+|\s+$//g;

        %hunk_toolchains = (%hunk_toolchains, %{ _detect_toolchains($content) });

        next if !_is_changed_line($line);
        next if $content eq q{} || $content =~ /^#/;

        push @changed_lines, $content;
        %changed_toolchains = (%changed_toolchains, %{ _detect_toolchains($content) });
    }

    return ({}, 0) if !@changed_lines;

    my $resolved_toolchains;
    if (!%changed_toolchains) {
        return ({}, 1) if scalar(keys %hunk_toolchains) != 1;
        $resolved_toolchains = { %hunk_toolchains };
    } else {
        $resolved_toolchains = { %changed_toolchains };
    }

    for my $content (@changed_lines) {
        return ({}, 1) if _touches_shared_ci_behavior($content);
        next if scalar(keys %{ _detect_toolchains($content) }) > 0;
        next if _is_toolchain_scoped_structural_line($content);
        return ({}, 1);
    }

    return ($resolved_toolchains, 0);
}

sub _detect_toolchains {
    my ($content) = @_;
    my $normalized = lc($content // q{});
    my %found;

    for my $toolchain (keys %TOOLCHAIN_MARKERS) {
        for my $marker (@{ $TOOLCHAIN_MARKERS{$toolchain} }) {
            if (index($normalized, $marker) >= 0) {
                $found{$toolchain} = 1;
                last;
            }
        }
    }

    return \%found;
}

sub _touches_shared_ci_behavior {
    my ($content) = @_;
    my $normalized = lc($content // q{});

    for my $marker (@UNSAFE_MARKERS) {
        return 1 if index($normalized, $marker) >= 0;
    }

    return 0;
}

sub _is_toolchain_scoped_structural_line {
    my ($content) = @_;

    for my $prefix (
        'if:',
        'run:',
        'shell:',
        'with:',
        'env:',
        '{',
        '}',
        'else',
        'fi',
        'then',
        'printf ',
        'echo ',
        'curl ',
        'powershell ',
        'call ',
        'cd ',
    ) {
        return 1 if index($content, $prefix) == 0;
    }

    return 0;
}

sub _is_diff_line {
    my ($line) = @_;
    return index($line, ' ') == 0 || _is_changed_line($line);
}

sub _is_changed_line {
    my ($line) = @_;
    return index($line, '+') == 0 || index($line, '-') == 0;
}

sub _file_diff {
    my ($root, $diff_base, $relative_path) = @_;

    for my $cmd (
        "git -C \\Q$root\\E diff --unified=0 ${diff_base}...HEAD -- $relative_path 2>/dev/null",
        "git -C \\Q$root\\E diff --unified=0 $diff_base HEAD -- $relative_path 2>/dev/null",
    ) {
        my $output = `$cmd`;
        return $output if $? == 0 && defined $output;
    }

    return q{};
}

1;
