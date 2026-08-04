var nul = "a\u0000b";
var esc = "a\u001bb";
var del = "a\u007fb";
var bs  = "a\u0008b";
var vt  = "a\u000bb";
var ff  = "a\u000cb";
sink(nul, esc, del, bs, vt, ff);
