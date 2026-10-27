library ieee;
use ieee.std_logic_1164.all;
package syntax_types is
  type mode_t is (idle, active);
end package;

entity syntax_tb is
end entity;

architecture sim of syntax_tb is
  signal flag : boolean := false;
  constant text : string := "-- psl assert never false;";
begin
  process(all)
  begin
    flag <= false;
    assert 1 + 2 = 3;
  end process;
end architecture;
