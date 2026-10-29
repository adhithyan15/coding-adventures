library ieee;
use ieee.std_logic_1164.all;

library work;
use work.common.all;

entity core_tb is
end entity core_tb;

architecture behave of core_tb is
    signal clk : std_ulogic := '0';
    signal rst : std_ulogic := '1';
    constant clk_period : time := 10 ns;
begin
    -- The source tree supplied to the generator is the immutable Microwatt
    -- commit recorded in rtl/manifest.txt.  FPU and BTC are deliberately off:
    -- neither facility belongs to Spec 07bd's scalar fixed-point profile.
    dut: entity work.soc
        generic map (
            SIM => true,
            MEMORY_SIZE => 384 * 1024,
            RAM_INIT_FILE => "main_ram.bin",
            CLK_FREQ => 100000000,
            HAS_FPU => false,
            HAS_BTC => false
        )
        port map (
            rst => rst,
            system_clk => clk
        );

    clk <= not clk after clk_period / 2;

    reset_process: process
    begin
        wait for 10 * clk_period;
        rst <= '0';
        wait;
    end process;
end architecture behave;
