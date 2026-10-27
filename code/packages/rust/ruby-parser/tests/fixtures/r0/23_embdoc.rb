# R1b: embedded documents, only at a line start
x = 1
=begin
puts 2
  =end is not the end, and neither is the next line
=endx
=end trailing text is ignored
y = x +
=begin
between an operator and its operand
=end
  2
=begin	a tab after the word
=end
z = y
