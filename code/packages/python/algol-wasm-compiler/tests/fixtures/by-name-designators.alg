begin
  integer result, flag;

  switch swDirectNameLeft := labelSwitchDirectNameLeft;
  switch swDirectNameRight := labelSwitchDirectNameRight;
  switch swDirectValueLeft := labelSwitchDirectValueLeft;
  switch swDirectValueRight := labelSwitchDirectValueRight;
  switch swRelayLeft := labelSwitchRelayLeft;
  switch swRelayRight := labelSwitchRelayRight;
  switch swFormalNameLeft := labelSwitchFormalNameLeft;
  switch swFormalNameRight := labelSwitchFormalNameRight;
  switch swFormalValueLeft := labelSwitchFormalValueLeft;
  switch swFormalValueRight := labelSwitchFormalValueRight;

  procedure jumpName(target);
    label target;
    begin flag := 1; goto target end;

  procedure jumpValue(target);
    value target;
    label target;
    begin flag := 1; goto target end;

  procedure relayLabel(target);
    label target;
    begin jumpName(target) end;

  procedure invokeLabelName(p);
    procedure p;
    begin p(if flag = 0 then labelFormalNameLeft else labelFormalNameRight) end;

  procedure invokeLabelValue(p);
    procedure p;
    begin p(if flag = 0 then labelFormalValueLeft else labelFormalValueRight) end;

  procedure switchName(sw);
    switch sw;
    begin flag := 1; goto sw[1] end;

  procedure switchValue(sw);
    value sw;
    switch sw;
    begin flag := 1; goto sw[1] end;

  procedure relaySwitch(sw);
    switch sw;
    begin switchName(sw) end;

  procedure invokeSwitchName(p);
    procedure p;
    begin p(if flag = 0 then swFormalNameLeft else swFormalNameRight) end;

  procedure invokeSwitchValue(p);
    procedure p;
    begin p(if flag = 0 then swFormalValueLeft else swFormalValueRight) end;

  result := 0;

  flag := 0;
  jumpName(if flag = 0 then labelDirectNameLeft else labelDirectNameRight);
labelDirectNameLeft:
  goto afterLabelDirectName;
labelDirectNameRight:
  result := result + 1;
afterLabelDirectName:
  flag := 0;
  jumpValue(if flag = 0 then labelDirectValueLeft else labelDirectValueRight);
labelDirectValueLeft:
  result := result + 10;
  goto afterLabelDirectValue;
labelDirectValueRight:
  goto afterLabelDirectValue;
afterLabelDirectValue:
  flag := 0;
  relayLabel(if flag = 0 then labelRelayLeft else labelRelayRight);
labelRelayLeft:
  goto afterLabelRelay;
labelRelayRight:
  result := result + 100;
afterLabelRelay:
  flag := 0;
  invokeLabelName(jumpName);
labelFormalNameLeft:
  goto afterLabelFormalName;
labelFormalNameRight:
  result := result + 1000;
afterLabelFormalName:
  flag := 0;
  invokeLabelValue(jumpValue);
labelFormalValueLeft:
  result := result + 10000;
  goto afterLabelFormalValue;
labelFormalValueRight:
  goto afterLabelFormalValue;
afterLabelFormalValue:
  flag := 0;
  switchName(if flag = 0 then swDirectNameLeft else swDirectNameRight);
labelSwitchDirectNameLeft:
  goto afterSwitchDirectName;
labelSwitchDirectNameRight:
  result := result + 100000;
afterSwitchDirectName:
  flag := 0;
  switchValue(if flag = 0 then swDirectValueLeft else swDirectValueRight);
labelSwitchDirectValueLeft:
  result := result + 1000000;
  goto afterSwitchDirectValue;
labelSwitchDirectValueRight:
  goto afterSwitchDirectValue;
afterSwitchDirectValue:
  flag := 0;
  relaySwitch(if flag = 0 then swRelayLeft else swRelayRight);
labelSwitchRelayLeft:
  goto afterSwitchRelay;
labelSwitchRelayRight:
  result := result + 10000000;
afterSwitchRelay:
  flag := 0;
  invokeSwitchName(switchName);
labelSwitchFormalNameLeft:
  goto afterSwitchFormalName;
labelSwitchFormalNameRight:
  result := result + 100000000;
afterSwitchFormalName:
  flag := 0;
  invokeSwitchValue(switchValue);
labelSwitchFormalValueLeft:
  result := result + 1000000000;
  goto afterSwitchFormalValue;
labelSwitchFormalValueRight:
  goto afterSwitchFormalValue;
afterSwitchFormalValue:
  print('DESIG');
  output(' ');
  print(result)
end
