begin
  integer result, ownScore, copyScore, controlScore, i, j;
  boolean array flags[1:1];
  string array words[1:1];

  procedure tick;
    begin
      own real scalar;
      own boolean seen;
      own string marker;
      own real array totals[1:1];
      own boolean array ready[1:1];
      own string array labels[1:1];

      if not seen then
        begin
          scalar := 1.5;
          seen := true;
          marker := 'OK';
          totals[1] := 2.5;
          ready[1] := true;
          labels[1] := 'ARR';
          ownScore := ownScore + 1
        end
      else
        begin
          scalar := scalar + 1.0;
          totals[1] := totals[1] + 1.0;
          if seen and ready[1] and (marker = 'OK') and (labels[1] = 'ARR') and
             (scalar > 2.0) and (totals[1] > 3.0) then
            ownScore := ownScore + 20
          else
            ownScore := 0
        end
    end;

  procedure mutate(flagCopy, wordCopy);
    value flagCopy, wordCopy;
    boolean flagCopy;
    string wordCopy;
    array flagCopy, wordCopy;
    begin
      flagCopy[1] := false;
      wordCopy[1] := 'copy';
      if (not flagCopy[1]) and (wordCopy[1] = 'copy') then
        copyScore := copyScore + 3
      else
        copyScore := 0
    end;

  procedure runByName(k);
    integer k;
    begin
      for k := 1 step 1 until 3 do controlScore := controlScore + k
    end;

  procedure runByValue(k);
    value k;
    integer k;
    begin
      for k := 1 step 1 until 3 do controlScore := controlScore + k
    end;

  ownScore := 0;
  copyScore := 0;
  controlScore := 0;
  tick;
  tick;

  flags[1] := true;
  words[1] := 'orig';
  mutate(flags, words);
  if flags[1] and (words[1] = 'orig') then copyScore := copyScore + 4
  else copyScore := 0;

  i := 0;
  runByName(i);
  controlScore := controlScore * 10 + i;
  j := 9;
  runByValue(j);
  controlScore := controlScore * 10 + j;

  result := ownScore + copyScore + controlScore;
  print('STORAGE');
  output(' ');
  print(result)
end
