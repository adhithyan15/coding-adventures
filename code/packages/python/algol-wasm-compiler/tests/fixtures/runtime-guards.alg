begin
  integer result;
  integer array a[1:2];

  result := 7;
  a[1] := 1;
  result := a[3];
  print('BAD')
end
