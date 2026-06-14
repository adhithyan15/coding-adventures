begin
  integer result, x, y, i, total;
  real scale;
  integer array a[1:3];
  string msg;

  integer procedure choose(flag, left, right);
    value flag, left, right;
    boolean flag;
    integer left, right;
    begin
      choose := if flag then left else right
    end;

  integer procedure sum(k, lo, hi, term);
    value lo, hi;
    integer k, lo, hi, term;
    begin
      sum := 0;
      for k := lo step 1 until hi do sum := sum + term
    end;

  ; msg := 'CONVERGE';
  x := y := 2;
  a[1] := x ** 3;
  a[2] := if y = 2 then 5 else 99;
  scale := 2.0 ^ (0 - 1);
  a[3] := choose(scale < 1.0, 7, 0);
  total := sum(i, 1, 3, a[i] * i);
  print(msg); output(' '); print(total);
  result := total;
end
