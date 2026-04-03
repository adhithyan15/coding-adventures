# cli-builder (Perl)

Declarative CLI argument parser driven by spec hash-refs.

## Usage

```perl
use CodingAdventures::CliBuilder;

my $spec = { cli_builder_spec_version => '1.0', name => 'myapp',
  description => 'My app', flags => [...], commands => [] };

my $result = CodingAdventures::CliBuilder->parse_hashref($spec, \@ARGV);
```

## Dependencies

None beyond core Perl modules.
