begin
  integer result, base;

  integer procedure addbase(x);
    value x;
    integer x;
    addbase := base + x;

  integer procedure applytwice(f, x);
    value x;
    integer f, x;
    procedure f;
    begin
      applytwice := f(x) + f(x + 1)
    end;

  base := 10;
  result := applytwice(addbase, 2);
  print('FORMAL');
  output(' ');
  print(result)
end
