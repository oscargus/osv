library std, work;
package definitions is
  constant supplied : integer := 1;
end package;

package body definitions is
end package body;

entity root is
end entity;

architecture sim of root is
  signal flag : boolean;
begin
  process
    variable flag : integer;
  begin
    wait;
  end process;
end architecture;
