begin integer result, i;
switch choose := first, second;
procedure escape;
begin goto done end;
result := 0;
i := 2;
goto choose[i];
first: result := 1; goto done;
second: begin i := i + 5; escape end;
done: print('FLOW'); output(' '); print(i); result := result + i
end
