begin
  integer result;
  boolean ready;
  string first, second;
  string array words[1:2];

  string procedure memo(s);
    value s;
    string s;
    begin
      own string saved;
      if saved = '' then saved := s;
      memo := saved
    end;

  ready := true;
  first := memo('READY');
  second := memo('OTHER');
  words[1] := first;
  words[2] := 'READY';
  result := 0;
  if ready = true then result := result + 1 else result := result + 100;
  if first = words[1] then result := result + 2 else result := result + 100;
  if words[1] = words[2] then result := result + 4 else result := result + 100;
  if second != 'OTHER' then result := result + 8 else result := result + 100;
  if words[2] != '' then result := result + 16 else result := result + 100;
  print(first);
  output(' ');
  print(result)
end
