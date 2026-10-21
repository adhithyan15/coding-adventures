var x = true;
function f() { switch (1) { case 1: if (x) break; console.log("g"); break; } console.log("h"); }
f();
