use strict;
use warnings;

use Test2::V0;
use lib 'lib';

use CodingAdventures::TypeCheckerProtocol qw(new_type_error_diagnostic new_type_check_result);

my $checker = CodingAdventures::TypeCheckerProtocol::GenericTypeChecker->new(
    node_kind => sub { $_[0]->{kind} },
    locate    => sub { return ($_[0]->{line}, $_[0]->{column}); },
);

$checker->register_hook('enter', 'fn decl', sub { return 'exact'; });
is($checker->dispatch('enter', { kind => 'fn decl' }), 'exact', 'exact hook dispatches');

my $fallback_checker = CodingAdventures::TypeCheckerProtocol::GenericTypeChecker->new(
    node_kind => sub { $_[0]->{kind} },
);
$fallback_checker->register_hook('enter', 'expr:add', sub { return $fallback_checker->not_handled; });
$fallback_checker->register_hook('enter', '*', sub { return 'fallback'; });
is($fallback_checker->dispatch('enter', { kind => 'expr:add' }), 'fallback', 'not_handled falls through');

my $result = $checker->check({ kind => 'expr', line => 6, column => 4 }, sub {
    my ($self, $ast) = @_;
    $self->error('bad node', $ast);
});

ok(!$result->{ok}, 'result reports errors');
is($result->{errors}[0]{message}, 'bad node', 'diagnostic message stored');
is($result->{errors}[0]{line}, 6, 'diagnostic line stored');
is($result->{errors}[0]{column}, 4, 'diagnostic column stored');

my $diagnostic = new_type_error_diagnostic('oops', 1, 2);
is($diagnostic, { message => 'oops', line => 1, column => 2 }, 'diagnostic constructor');

my $clean = new_type_check_result({ ast => 1 }, []);
ok($clean->{ok}, 'clean result is ok');

done_testing;
