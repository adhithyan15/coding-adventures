library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
use std.env.all;

use work.zpu_config.all;
use work.zpupkg.all;
use work.oracle_control.all;

entity dualport_ram is
  port (
    clk             : in  std_logic;
    memAWriteEnable : in  std_logic;
    memAAddr        : in  std_logic_vector(maxAddrBitBRAM downto minAddrBit);
    memAWrite       : in  std_logic_vector(wordSize - 1 downto 0);
    memARead        : out std_logic_vector(wordSize - 1 downto 0);
    memBWriteEnable : in  std_logic;
    memBAddr        : in  std_logic_vector(maxAddrBitBRAM downto minAddrBit);
    memBWrite       : in  std_logic_vector(wordSize - 1 downto 0);
    memBRead        : out std_logic_vector(wordSize - 1 downto 0)
  );
end entity;

architecture oracle of dualport_ram is
  type memory_type is array (0 to 8191) of std_logic_vector(31 downto 0);
  signal a_index : natural range 0 to 8191;
  signal b_index : natural range 0 to 8191;
begin
  a_index <= to_integer(unsigned(memAAddr));
  b_index <= to_integer(unsigned(memBAddr));

  process (clk, load_case)
    variable fresh : memory_type;
    variable memory : memory_type := (others => x"0b0b0b0b");
  begin
    if load_case = '1' then
      fresh := (others => x"0b0b0b0b");
      for offset in 0 to 31 loop
        fresh((8190 + offset) mod 8192) :=
          std_logic_vector(to_unsigned(16#40000000# + offset, 32));
      end loop;
      fresh(64) := x"a5a55a5a";
      fresh(8190) := x"00000100";
      fresh(8191) := x"11223344";
      fresh(0) := case_raw & x"0b0b0b";
      if scenario = 1 then
        fresh(8190) := x"00008000";
      elsif scenario = 2 then
        fresh(8190) := x"00008000";
      elsif scenario = 3 or scenario = 4 then
        fresh(0) := x"81820b0b";
      end if;
      memory := fresh;
    elsif rising_edge(clk) then
      if memAWriteEnable = '1' and memBWriteEnable = '1' and
        a_index = b_index and memAWrite /= memBWrite
      then
        report "oracle dual-port write collision" severity error;
        stop(1);
      end if;

      if memAWriteEnable = '1' then
        report "ZPU_RAM_WRITE " & integer'image(case_number) &
          " A " & integer'image(a_index) & " " & to_hstring(memAWrite)
          severity note;
        memory(a_index) := memAWrite;
        memARead <= memAWrite;
      else
        memARead <= memory(a_index);
      end if;
      if memBWriteEnable = '1' then
        report "ZPU_RAM_WRITE " & integer'image(case_number) &
          " B " & integer'image(b_index) & " " & to_hstring(memBWrite)
          severity note;
        memory(b_index) := memBWrite;
        memBRead <= memBWrite;
      else
        memBRead <= memory(b_index);
      end if;
    end if;
  end process;
end architecture;
