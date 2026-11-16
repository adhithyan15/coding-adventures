use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LunarLander qw(
    new_flight
    validate_burn
    check_burn
    step_burn
    div_euclid
    isqrt_u128
    to_paint_scene
    telemetry_scene
    render_telemetry_ascii
    render_telemetry
    format_with_commas
);

ok(defined &new_flight,             'new_flight is exported');
ok(defined &validate_burn,          'validate_burn is exported');
ok(defined &check_burn,             'check_burn is exported');
ok(defined &step_burn,              'step_burn is exported');
ok(defined &div_euclid,             'div_euclid is exported');
ok(defined &isqrt_u128,             'isqrt_u128 is exported');
ok(defined &to_paint_scene,         'to_paint_scene is exported');
ok(defined &telemetry_scene,        'telemetry_scene is exported');
ok(defined &render_telemetry_ascii, 'render_telemetry_ascii is exported');
ok(defined &render_telemetry,       'render_telemetry is exported');
ok(defined &format_with_commas,     'format_with_commas is exported');

is($CodingAdventures::LunarLander::VERSION, '0.1.0', 'version is 0.1.0');

done_testing;
