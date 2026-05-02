begin
  integer result, lo, hi;

  lo := 2;
  hi := 4;
  begin
    integer array a[lo:hi, 1:2];
    lo := 99;
    hi := 100;
    a[2, 1] := 21;
    a[4, 2] := 42;
    result := a[2, 1] + a[4, 2]
  end;

  print('DYNAMIC');
  output(' ');
  print(result)
end
