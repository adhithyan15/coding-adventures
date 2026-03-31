use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmModuleParser qw(parse parse_header parse_section get_section);

ok(1, 'module loads');
is($CodingAdventures::WasmModuleParser::VERSION, '0.01', 'has VERSION 0.01');

# Verify section ID constants are exported
is(CodingAdventures::WasmModuleParser::SECTION_CUSTOM(),   0,  'SECTION_CUSTOM = 0');
is(CodingAdventures::WasmModuleParser::SECTION_TYPE(),     1,  'SECTION_TYPE = 1');
is(CodingAdventures::WasmModuleParser::SECTION_IMPORT(),   2,  'SECTION_IMPORT = 2');
is(CodingAdventures::WasmModuleParser::SECTION_FUNCTION(), 3,  'SECTION_FUNCTION = 3');
is(CodingAdventures::WasmModuleParser::SECTION_TABLE(),    4,  'SECTION_TABLE = 4');
is(CodingAdventures::WasmModuleParser::SECTION_MEMORY(),   5,  'SECTION_MEMORY = 5');
is(CodingAdventures::WasmModuleParser::SECTION_GLOBAL(),   6,  'SECTION_GLOBAL = 6');
is(CodingAdventures::WasmModuleParser::SECTION_EXPORT(),   7,  'SECTION_EXPORT = 7');
is(CodingAdventures::WasmModuleParser::SECTION_START(),    8,  'SECTION_START = 8');
is(CodingAdventures::WasmModuleParser::SECTION_ELEMENT(),  9,  'SECTION_ELEMENT = 9');
is(CodingAdventures::WasmModuleParser::SECTION_CODE(),    10,  'SECTION_CODE = 10');
is(CodingAdventures::WasmModuleParser::SECTION_DATA(),    11,  'SECTION_DATA = 11');

# Verify module constants
is(CodingAdventures::WasmModuleParser::MODULE_MAGIC(),   "\x00asm", 'MODULE_MAGIC');
is(CodingAdventures::WasmModuleParser::MODULE_VERSION(), 1,          'MODULE_VERSION');

done_testing;
