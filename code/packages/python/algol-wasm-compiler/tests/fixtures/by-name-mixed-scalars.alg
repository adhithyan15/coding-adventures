begin
  integer result;
  real r;
  boolean b;
  string s;

  procedure setall(x, y, z);
    real x;
    boolean y;
    string z;
    begin
      x := x + 0.5;
      y := not y;
      z := 'done'
    end;

  r := 1.5;
  b := true;
  s := 'todo';
  setall(r, b, s);
  if (r = 2.0) and (not b) and (s = 'done') then
    result := 27
  else
    result := 0;

  print('BYNAME');
  output(' ');
  print(result)
end
