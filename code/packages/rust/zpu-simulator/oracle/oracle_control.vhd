library ieee;
use ieee.std_logic_1164.all;

package oracle_control is
  signal load_case : std_logic := '0';
  signal case_raw  : std_logic_vector(7 downto 0) := (others => '0');
  signal case_number : natural range 0 to 260 := 0;
  signal scenario    : natural range 0 to 4 := 0;
end package;
