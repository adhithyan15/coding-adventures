begin own integer counter; integer array a[1:3]; real average;
string msg; integer result;
integer procedure inc(x); value x; integer x;
begin inc := x + 1 end;
procedure bump;
begin counter := counter + 1 end;
msg := 'ALGOL';
a[1] := 2;
a[2] := inc(4);
a[3] := 7;
bump;
bump;
average := (a[1] + a[2] + a[3]) / 2;
print(msg);
output(' ');
print(counter);
output(' ');
print(average);
result := a[2] + a[3]
end
