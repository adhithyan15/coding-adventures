begin
  integer result, calls;

  integer procedure fact(n);
    value n;
    integer n;
    begin
      calls := calls + 1;
      if n <= 1 then
        fact := 1
      else
        fact := n * fact(n - 1)
    end;

  calls := 0;
  result := fact(5) + calls;
  print('RECURSE');
  output(' ');
  print(result)
end
