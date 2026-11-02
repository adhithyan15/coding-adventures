library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
use std.env.all;

use work.zpu_config.all;
use work.zpupkg.all;
use work.oracle_control.all;

entity zpu_oracle_tb is
end entity;

architecture test of zpu_oracle_tb is
  signal clk                  : std_logic := '0';
  signal reset                : std_logic := '1';
  signal external_busy        : std_logic := '0';
  signal external_read        : std_logic_vector(31 downto 0) := x"89abcdef";
  signal external_write       : std_logic_vector(31 downto 0);
  signal external_address     : std_logic_vector(15 downto 0);
  signal external_write_enable : std_logic;
  signal external_read_enable  : std_logic;
  signal external_write_mask   : std_logic_vector(3 downto 0);
  signal interrupt             : std_logic := '0';
  signal break_asserted        : std_logic;
begin
  clk <= not clk after 5 ns;

  core : entity work.zpu_core
    port map (
      clk                 => clk,
      reset               => reset,
      enable              => '1',
      in_mem_busy         => external_busy,
      mem_read            => external_read,
      mem_write           => external_write,
      out_mem_addr        => external_address,
      out_mem_writeEnable => external_write_enable,
      out_mem_readEnable  => external_read_enable,
      mem_writeMask       => external_write_mask,
      interrupt           => interrupt,
      break               => break_asserted
    );

  stimulus : process
  begin
    for raw in 0 to 255 loop
      case_number <= raw;
      scenario <= 0;
      case_raw <= std_logic_vector(to_unsigned(raw, 8));
      reset <= '1';
      load_case <= '1';
      wait for 11 ns;
      load_case <= '0';
      wait for 6 ns;
      reset <= '0';
      wait for 183 ns;
      report "ZPU_CASE_DONE " & integer'image(raw) severity note;
    end loop;
    for special in 1 to 4 loop
      case_number <= 255 + special;
      scenario <= special;
      if special = 1 then
        case_raw <= x"08";
      elsif special = 2 then
        case_raw <= x"0c";
      else
        case_raw <= x"81";
      end if;
      interrupt <= '0';
      reset <= '1';
      load_case <= '1';
      wait for 11 ns;
      load_case <= '0';
      wait for 6 ns;
      reset <= '0';
      if special = 4 then
        wait for 53 ns;
        interrupt <= '1';
        wait for 190 ns;
      else
        wait for 243 ns;
      end if;
      report "ZPU_CASE_DONE " & integer'image(255 + special) severity note;
    end loop;
    interrupt <= '0';
    report "ZPU_ORACLE_PASS" severity note;
    stop(0);
    wait;
  end process;

  transactions : process (clk)
  begin
    if rising_edge(clk) then
      if external_read_enable = '1' then
        report "ZPU_BUS_READ " &
          integer'image(case_number) & " " &
          to_hstring(external_address) & " " & to_hstring(external_read)
          severity note;
      end if;
      if external_write_enable = '1' then
        report "ZPU_BUS_WRITE " &
          integer'image(case_number) & " " &
          to_hstring(external_address) & " " & to_hstring(external_write)
          severity note;
      end if;
    end if;
  end process;
end architecture;
