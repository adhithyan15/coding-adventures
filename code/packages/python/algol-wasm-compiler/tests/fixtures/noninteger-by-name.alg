begin
  integer result;
  real r;
  real array reals[1:2];
  boolean flag;
  boolean array bools[1:2];
  string word, seen;
  string array words[1:2];

  procedure bumpreal(x);
    real x;
    begin
      x := x + 1.5
    end;

  real procedure readreal(x);
    real x;
    begin
      r := r + 1.0;
      readreal := x
    end;

  procedure flipbool(x);
    boolean x;
    begin
      x := not x
    end;

  boolean procedure readbool(x);
    boolean x;
    begin
      flag := not flag;
      readbool := x
    end;

  procedure setstring(x);
    string x;
    begin
      x := 'OK'
    end;

  string procedure readstring(x);
    string x;
    begin
      word := 'LAZY';
      readstring := x
    end;

  procedure applyreal(p, x);
    procedure p;
    real x;
    begin
      p(x)
    end;

  procedure applybool(p, x);
    procedure p;
    boolean x;
    begin
      p(x)
    end;

  procedure applystring(p, x);
    procedure p;
    string x;
    begin
      p(x)
    end;

  result := 0;

  r := 1.0;
  reals[1] := 2.0;
  applyreal(bumpreal, r);
  if r = 2.5 then result := result + 1 else result := 0;
  applyreal(bumpreal, reals[1]);
  if reals[1] = 3.5 then result := result + 10 else result := 0;
  if readreal(r + reals[1]) = 7.0 then result := result + 100 else result := 0;

  flag := true;
  bools[1] := false;
  applybool(flipbool, flag);
  if not flag then result := result + 1000 else result := 0;
  applybool(flipbool, bools[1]);
  if bools[1] then result := result + 10000 else result := 0;
  flag := false;
  if readbool(flag) then result := result + 100000 else result := 0;

  word := 'NO';
  words[1] := 'WAIT';
  applystring(setstring, word);
  if word = 'OK' then result := result + 1000000 else result := 0;
  applystring(setstring, words[1]);
  if words[1] = 'OK' then result := result + 10000000 else result := 0;
  word := 'EARLY';
  seen := readstring(if true then word else 'NO');
  if seen = 'LAZY' then result := result + 100000000 else result := 0;

  print('TYPED');
  output(' ');
  print(seen);
  output(' ');
  print(result)
end
