begin
  own integer counter;
  own integer array hits[1:2];
  integer result, i, total, loops, switches;
  real scale;
  boolean flag;
  string msg;
  integer array a[1:4], m[1:2, 1:2];
  array weights[1:2];

  integer procedure inc(x);
    value x;
    integer x;
    inc := x + 1;

  procedure bump;
    begin counter := counter + 1; hits[counter] := counter end;

  integer procedure twice(x);
    integer x;
    begin twice := x + x end;

  integer procedure apply(p, x);
    value x;
    integer p, x;
    procedure p;
    begin apply := p(x) end;

  integer procedure fold(k, lo, hi, term);
    value lo, hi;
    integer k, lo, hi, term;
    begin
      fold := 0;
      for k := lo step 1 until hi do fold := fold + term
    end;

  procedure jumpVia(s, fallback);
    switch s;
    label fallback;
    begin
      if counter = 0 then goto fallback else goto s[1]
    end;

  switch route := (if flag then left else right), 90;

  msg := 'COMPLETE';
  flag := true;
  result := total := 0;
  scale := 2.0 ^ (0 - 1);
  weights[1] := scale + 1.0;
  weights[2] := weights[1] + 0.5;
  a[1] := 1;
  a[2] := if flag then inc(a[1]) else 0;
  a[3] := 3;
  a[4] := 4;
  m[1, 1] := 5;
  m[1, 2] := 6;
  m[2, 1] := 7;
  m[2, 2] := 8;
  bump;
  bump;
  total := fold(i, 1, 4, a[i] * i);
  total := total + apply(twice, hits[2]);
  total := total + entier(weights[2] * 10);
  loops := 0;
  for i := 1, 3 step 1 until 4, 10 while loops < 3 do
    begin loops := loops + i end;
  total := total + loops;
  switches := 0;
  jumpVia(route, fallback);
  switches := 99;
  goto afterSwitch;
fallback:
  switches := 3;
  goto afterSwitch;
left:
  switches := 11;
  go to afterSwitch;
right:
  switches := 17;
  goto afterSwitch;
90:
  switches := 90;
afterSwitch:
  total := total + switches;
  flag := (total = 73) and ((counter = 2) impl true) and (true eqv true) and not false;
  if flag or false then result := total + m[2, 2] else result := 0;
  print(msg);
  output(' ');
  print(result);
done:
end
