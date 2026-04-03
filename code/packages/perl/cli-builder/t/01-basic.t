use strict;
use warnings;
use Test2::V0;

require CodingAdventures::CliBuilder;
my $pkg = 'CodingAdventures::CliBuilder';

# ============================================================================
# Shared test specs
# ============================================================================

my $ECHO_SPEC = {
  cli_builder_spec_version => '1.0',
  name        => 'echo',
  description => 'Print text to stdout',
  version     => '1.0.0',
  builtin_flags => { help => 1, version => 1 },
  flags => [
    { id => 'newline', short => 'n', long => 'newline',
      description => 'Suppress newline', type => 'boolean' },
    { id => 'count', short => 'c', long => 'count',
      description => 'Repeat count', type => 'integer', default => 1 },
  ],
  arguments => [
    { id => 'string', display_name => 'STRING',
      description => 'Text to print', required => 1, variadic => 1 },
  ],
  commands => [],
};

my $GIT_SPEC = {
  cli_builder_spec_version => '1.0',
  name        => 'git',
  description => 'The git VCS',
  version     => '2.40.0',
  builtin_flags => { help => 1, version => 1 },
  global_flags => [
    { id => 'verbose', short => 'v', long => 'verbose',
      description => 'Verbose output', type => 'boolean' },
  ],
  flags     => [],
  arguments => [],
  commands  => [
    {
      name => 'commit',
      description => 'Record changes',
      flags => [
        { id => 'message', short => 'm', long => 'message',
          description => 'Commit message', type => 'string', required => 1 },
        { id => 'amend', long => 'amend', description => 'Amend last commit',
          type => 'boolean' },
      ],
      arguments => [],
      commands  => [],
    },
    {
      name => 'remote',
      description => 'Manage remotes',
      flags     => [],
      arguments => [],
      commands  => [
        {
          name => 'add',
          description => 'Add a remote',
          flags     => [],
          arguments => [
            { id => 'name', display_name => 'NAME', description => 'Remote name', required => 1 },
            { id => 'url',  display_name => 'URL',  description => 'Remote URL',  required => 1 },
          ],
          commands => [],
        },
      ],
    },
  ],
};

my $ENUM_SPEC = {
  cli_builder_spec_version => '1.0',
  name => 'format',
  description => 'Format converter',
  flags => [
    { id => 'output', short => 'o', long => 'output',
      description => 'Output format', type => 'enum',
      enum_values => ['json', 'csv', 'xml'] },
  ],
  arguments => [],
  commands  => [],
};

# ============================================================================
# TokenClassifier
# ============================================================================

subtest 'TokenClassifier' => sub {
  my $tc = 'CodingAdventures::CliBuilder::TokenClassifier';
  my @flags = (
    { id=>'verbose', short=>'v', long=>'verbose', single_dash_long=>undef, type=>'boolean' },
    { id=>'output',  short=>'o', long=>'output',  single_dash_long=>undef, type=>'string'  },
    { id=>'count',   short=>'c', long=>'count',   single_dash_long=>undef, type=>'count'   },
    { id=>'cp',      short=>undef, long=>undef,   single_dash_long=>'classpath', type=>'string' },
  );

  is($tc->classify('--', \@flags)->{kind}, 'end_of_flags', '-- is end_of_flags');

  my $t = $tc->classify('--verbose', \@flags);
  is($t->{kind}, 'long_flag', '--verbose kind');
  is($t->{name}, 'verbose',   '--verbose name');

  $t = $tc->classify('--output=foo.txt', \@flags);
  is($t->{kind},  'long_flag_value', '--output=foo.txt kind');
  is($t->{name},  'output',          '--output=foo.txt name');
  is($t->{value}, 'foo.txt',         '--output=foo.txt value');

  $t = $tc->classify('-v', \@flags);
  is($t->{kind}, 'short_flag', '-v kind');
  is($t->{char}, 'v',          '-v char');

  $t = $tc->classify('-ofile.txt', \@flags);
  is($t->{kind},  'short_flag_value', '-ofile.txt kind');
  is($t->{char},  'o',                '-ofile.txt char');
  is($t->{value}, 'file.txt',         '-ofile.txt value');

  $t = $tc->classify('-vc', \@flags);
  is($t->{kind}, 'stacked_flags', '-vc is stacked');
  ok(ref($t->{chars}) eq 'ARRAY', '-vc chars is array');

  $t = $tc->classify('hello', \@flags);
  is($t->{kind},  'positional', 'hello kind');
  is($t->{value}, 'hello',      'hello value');

  is($tc->classify('-', \@flags)->{kind}, 'positional', '- is positional');

  $t = $tc->classify('-classpath', \@flags);
  is($t->{kind}, 'single_dash_long', '-classpath is SDL');
  is($t->{name}, 'classpath',        '-classpath name');

  is($tc->classify('-z', \@flags)->{kind}, 'unknown_flag', '-z is unknown');
};

# ============================================================================
# SpecLoader
# ============================================================================

subtest 'SpecLoader' => sub {
  my $sl = 'CodingAdventures::CliBuilder::SpecLoader';

  my $spec = $sl->load_hashref($ECHO_SPEC);
  is(ref $spec, 'HASH', 'returns hash-ref');
  is($spec->{name}, 'echo', 'name');
  is($spec->{version}, '1.0.0', 'version');
  is(scalar @{$spec->{flags}}, 2, '2 flags');
  is($spec->{flags}[0]{id}, 'newline', 'first flag id');
  is($spec->{flags}[0]{type}, 'boolean', 'type');
  is(scalar @{$spec->{arguments}}, 1, '1 argument');
  ok($spec->{arguments}[0]{variadic}, 'argument is variadic');
  is($spec->{parsing_mode}, 'gnu', 'default parsing_mode');

  ok( dies { $sl->load_hashref({ cli_builder_spec_version => '1.0', description => 'test' }) },
      'missing name dies' );
  ok( dies { $sl->load_hashref({ cli_builder_spec_version => '99', name => 'x', description => 'y' }) },
      'bad version dies' );
  ok( dies { $sl->load_hashref({ cli_builder_spec_version => '1.0', name => 'x', description => 'y',
      flags => [{ description => 'x', type => 'boolean' }] }) },
      'flag missing id dies' );
  ok( dies { $sl->load_hashref({ cli_builder_spec_version => '1.0', name => 'x', description => 'y',
      flags => [{ id => 'x', description => 'y', type => 'invalid_type' }] }) },
      'invalid type dies' );
  ok( dies { $sl->load_hashref({ cli_builder_spec_version => '1.0', name => 'x', description => 'y',
      flags => [{ id => 'x', description => 'y', type => 'enum' }] }) },
      'enum without enum_values dies' );
};

# ============================================================================
# HelpGenerator
# ============================================================================

subtest 'HelpGenerator' => sub {
  my $sl   = 'CodingAdventures::CliBuilder::SpecLoader';
  my $hg   = 'CodingAdventures::CliBuilder::HelpGenerator';
  my $spec = $sl->load_hashref($ECHO_SPEC);

  my $text = $hg->generate($spec, ['echo']);
  ok(length($text) > 0, 'help text not empty');
  ok($text =~ /USAGE/,       'has USAGE section');
  ok($text =~ /echo/,        'contains program name');
  ok($text =~ /DESCRIPTION/, 'has DESCRIPTION section');
};

# ============================================================================
# Parser — basic flags
# ============================================================================

subtest 'Parser boolean flag' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['--newline', 'hello']);
  is($r->{type}, 'result', 'type=result');
  ok($r->{flags}{newline}, '--newline sets true');
};

subtest 'Parser short boolean flag' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['-n', 'hello']);
  is($r->{type}, 'result', 'type=result');
  ok($r->{flags}{newline}, '-n sets true');
};

subtest 'Parser integer flag --count=3' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['--count=3', 'hello']);
  is($r->{type}, 'result', 'type=result');
  is($r->{flags}{count}, 3, 'count=3');
};

subtest 'Parser short flag with separate value' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['-c', '5', 'hello']);
  is($r->{type}, 'result', 'type=result');
  is($r->{flags}{count}, 5, 'count=5');
};

subtest 'Parser absent boolean flag defaults to false' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['hello']);
  is($r->{type}, 'result', 'type=result');
  ok(!$r->{flags}{newline}, 'newline=false');
};

subtest 'Parser absent integer uses default' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['hello']);
  is($r->{flags}{count}, 1, 'default count=1');
};

subtest 'Parser -- makes subsequent tokens positional' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['--', '--not-a-flag']);
  is($r->{type}, 'result', 'type=result');
  ok(defined $r->{arguments}{string}, 'positional captured');
};

# ============================================================================
# Parser — positional arguments
# ============================================================================

subtest 'Parser variadic argument' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['hello', 'world']);
  is($r->{type}, 'result', 'type=result');
  is(ref $r->{arguments}{string}, 'ARRAY', 'variadic is array');
  is(scalar @{$r->{arguments}{string}}, 2, '2 positionals');
  is($r->{arguments}{string}[0], 'hello', 'first positional');
  is($r->{arguments}{string}[1], 'world', 'second positional');
};

subtest 'Parser missing required argument' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, []);
  is($r->{type}, 'error', 'type=error');
  my $has = grep { $_->{error_type} eq 'missing_argument' } @{$r->{errors}};
  ok($has, 'missing_argument error');
};

# ============================================================================
# Parser — subcommands
# ============================================================================

subtest 'Parser commit subcommand' => sub {
  my $r = $pkg->parse_hashref($GIT_SPEC, ['commit', '-m', 'Initial commit']);
  is($r->{type}, 'result', 'type=result');
  is($r->{command_path}[1], 'commit', 'routes to commit');
  is($r->{flags}{message}, 'Initial commit', 'message flag');
};

subtest 'Parser nested subcommand remote add' => sub {
  my $r = $pkg->parse_hashref($GIT_SPEC,
    ['remote', 'add', 'origin', 'https://github.com/user/repo']);
  is($r->{type}, 'result', 'type=result');
  is($r->{command_path}[2], 'add', 'routes to add');
  is($r->{arguments}{name}, 'origin', 'name argument');
  is($r->{arguments}{url},  'https://github.com/user/repo', 'url argument');
};

subtest 'Parser global flag in subcommand' => sub {
  my $r = $pkg->parse_hashref($GIT_SPEC, ['--verbose', 'commit', '-m', 'msg']);
  is($r->{type}, 'result', 'type=result');
  ok($r->{flags}{verbose}, 'verbose=true');
};

subtest 'Parser missing required subcommand flag' => sub {
  my $r = $pkg->parse_hashref($GIT_SPEC, ['commit']);
  is($r->{type}, 'error', 'type=error');
  my $has = grep { $_->{error_type} eq 'flag_error' && $_->{message} =~ /message/ }
            @{$r->{errors}};
  ok($has, 'required message flag error');
};

# ============================================================================
# Parser — help and version
# ============================================================================

subtest 'Parser --help returns help' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['--help']);
  is($r->{type}, 'help', 'type=help');
  ok(length($r->{text}) > 0, 'has text');
  is(ref $r->{command_path}, 'ARRAY', 'has command_path');
};

subtest 'Parser -h returns help' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['-h']);
  is($r->{type}, 'help', 'type=help');
};

subtest 'Parser --version returns version' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['--version']);
  is($r->{type}, 'version', 'type=version');
  is($r->{version}, '1.0.0', 'version string');
};

# ============================================================================
# Parser — enum validation
# ============================================================================

subtest 'Parser valid enum' => sub {
  my $r = $pkg->parse_hashref($ENUM_SPEC, ['--output=json']);
  is($r->{type}, 'result', 'type=result');
  is($r->{flags}{output}, 'json', 'output=json');
};

subtest 'Parser invalid enum' => sub {
  my $r = $pkg->parse_hashref($ENUM_SPEC, ['--output=pdf']);
  is($r->{type}, 'error', 'type=error');
  my $has = grep { $_->{error_type} eq 'flag_error' && $_->{message} =~ /invalid value/ }
            @{$r->{errors}};
  ok($has, 'invalid value error');
};

# ============================================================================
# Parser — unknown flags
# ============================================================================

subtest 'Parser unknown long flag' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['--unknown-flag']);
  is($r->{type}, 'error', 'type=error');
  my $has = grep { $_->{error_type} eq 'unknown_flag' } @{$r->{errors}};
  ok($has, 'unknown_flag error');
};

subtest 'Parser unknown short flag' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['-z']);
  is($r->{type}, 'error', 'type=error');
};

# ============================================================================
# Parser — missing flag value
# ============================================================================

subtest 'Parser flag at end of argv' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['--count']);
  is($r->{type}, 'error', 'type=error');
  my $has = grep { $_->{error_type} eq 'missing_flag_value' } @{$r->{errors}};
  ok($has, 'missing_flag_value error');
};

# ============================================================================
# Parser — result structure
# ============================================================================

subtest 'Parser result structure' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['hello']);
  is($r->{type}, 'result', 'type=result');
  is($r->{program}, 'echo', 'program=echo');
  is($r->{command_path}[0], 'echo', 'command_path[0]=echo');
  is(ref $r->{flags},          'HASH',  'flags is hash');
  is(ref $r->{arguments},      'HASH',  'arguments is hash');
  is(ref $r->{explicit_flags}, 'ARRAY', 'explicit_flags is array');
};

subtest 'Parser explicit_flags lists set flags' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['--newline', 'hello']);
  is($r->{type}, 'result', 'type=result');
  my $has = grep { $_ eq 'newline' } @{$r->{explicit_flags}};
  ok($has, 'newline in explicit_flags');
};

subtest 'Parser explicit_flags omits defaulted flags' => sub {
  my $r = $pkg->parse_hashref($ECHO_SPEC, ['hello']);
  my $has = grep { $_ eq 'newline' } @{$r->{explicit_flags}};
  ok(!$has, 'newline not in explicit_flags');
};

done_testing;
