begin integer result, i; integer array a[1:4];
integer procedure sum(k, lo, hi, term);
value lo, hi; integer k, lo, hi, term;
begin sum := 0; for k := lo step 1 until hi do sum := sum + term end;
a[1] := 1;
a[2] := 2;
a[3] := 3;
a[4] := 4;
result := sum(i, 1, 4, a[i] * i)
end
