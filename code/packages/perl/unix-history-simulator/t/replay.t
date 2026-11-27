use strict;
use warnings;
use FindBin;
use lib "$FindBin::Bin/../lib";
use JSON::PP qw(decode_json);
use Test::More;
use CodingAdventures::UnixHistory;

my $fixture_path = "$FindBin::Bin/../../../../specs/fixtures/historical-unix-v1/file-read.json";
open my $fixture, '<:raw', $fixture_path or die "$fixture_path: $!";
my $cases = do { local $/; decode_json(<$fixture>) };
close $fixture;
my $files = CodingAdventures::UnixHistory::FileSystem->demo;
is_deeply([$files->read(2, -1, 1)], [undef, 'EINVAL'], 'negative byte offset rejected');
is_deeply([$files->read(2, 0, -1)], [undef, 'EINVAL'], 'negative byte count rejected');
is_deeply([$files->read(2, 0.5, 1)], [undef, 'EINVAL'], 'fractional byte offset rejected');
$files->{2}{data} = "\x{e9}";
my ($utf8_bytes, $range_error) = $files->read(2, 0, 2);
is($range_error, undef, 'UTF-8 byte range accepted');
is(unpack('H*', $utf8_bytes), 'c3a9', 'file reads count UTF-8 bytes, not characters');
for my $case (@$cases) {
    my ($command, $output, $events) = @$case{qw(command output events)};
    is_deeply(CodingAdventures::UnixHistory::run_command($command),
        { output => $output, events => $events }, "replays '$command'");
}

my $table = CodingAdventures::UnixHistory::Descriptors->new(2);
is($table->allocate(2), 0, 'lowest slot allocated');
is($table->allocate(2), 1, 'second open uses second slot');
is($table->allocate(2), undef, 'full table');
ok($table->advance(0, 4), 'offset advanced');
is($table->get(1)->{offset}, 0, 'other descriptor has an independent offset');
ok(!$table->close(-1), 'negative descriptor rejected');
ok($table->close(0), 'descriptor released');
is($table->allocate(3), 0, 'freed slot is reused');

done_testing;
