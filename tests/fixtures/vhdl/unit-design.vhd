entity unit_tb is
  constant threshold : integer := 3;
end entity;

architecture sim of unit_tb is
  constant limit : integer := threshold + 1;
  signal done : boolean := false;
  signal count : integer;
begin
  worker: process
  begin
    wait;
  end process;
end architecture;
