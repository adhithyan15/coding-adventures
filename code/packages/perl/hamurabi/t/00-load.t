use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Hamurabi qw(
    new_game
    validate_order
    check_order
    step_year
    format_report
);

ok(defined &new_game,       'new_game is exported');
ok(defined &validate_order, 'validate_order is exported');
ok(defined &check_order,    'check_order is exported');
ok(defined &step_year,      'step_year is exported');
ok(defined &format_report,  'format_report is exported');

is($CodingAdventures::Hamurabi::VERSION, '0.1.0', 'version is 0.1.0');

done_testing;
