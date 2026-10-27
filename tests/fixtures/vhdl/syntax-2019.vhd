package mode_views is
  type pair is record
    value : bit;
  end record;
  view read_only of pair is
    value : in;
  end view;
end package;
