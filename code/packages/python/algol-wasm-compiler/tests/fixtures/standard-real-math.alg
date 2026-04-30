begin
  integer result;
  real metric;
  string msg;

  msg := 'MATH';
  metric := sin(0) + cos(0) + arctan(1) + ln(exp(1));
  result := entier(metric * 100);
  print(msg); output(' '); print(result);
end
