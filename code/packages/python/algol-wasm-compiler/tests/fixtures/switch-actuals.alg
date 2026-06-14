begin
  integer result, phase;
  boolean pick;

  switch a := alabel;
  switch b := blabel;

  procedure jump(sw);
    switch sw;
    begin goto sw[1] end;

  procedure relay(x, y);
    switch x, y;
    begin jump(if pick then x else y) end;

  procedure invoke(p);
    procedure p;
    begin p(if pick then a else b) end;

  result := 0;
  phase := 1;
  pick := false;
  jump(if pick then a else b);

alabel:
  result := result + phase;
  goto after;

blabel:
  result := result + phase * 10;
  goto after;

after:
  if phase = 1 then
    begin
      phase := 2;
      pick := true;
      relay(a, b)
    end
  else if phase = 2 then
    begin
      phase := 3;
      pick := false;
      invoke(jump)
    end
  else
    goto done;

done:
  print('SWITCH');
  output(' ');
  print(result)
end
