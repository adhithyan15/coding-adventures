begin
  integer result, edgeScore, subscriptScore, labelScore;
  boolean choose, ok;
  string word;
  integer array values[1:2];

  choose := false;
  ok := if choose then false else true;
  word := if ok then 'YES' else 'NO';
  if ok and (word = 'YES') then edgeScore := 11 else edgeScore := 0;

  values[1] := 3;
  values[2] := 5;
  subscriptScore := values[if ok then 2 else 1] * 7;

  begin
    labelScore := 19;
    goto finished;
    labelScore := 0;
    finished:
  end;

  result := edgeScore + subscriptScore + labelScore;
  print('EDGE');
  output(' ');
  print(result)
end
