begin
  integer result, n;

  procedure escape;
    begin
      integer array temp[1:n];
      temp[n] := 7;
      goto after
    end;

  result := 0;
  n := 20;
  escape;
  result := 99;

after:
  n := 2;
  begin
    integer array next[1:n];
    next[2] := 41;
    result := next[2] + 1
  end;

  print('UNWIND');
  output(' ');
  print(result)
end
