const T = true; [1].forEach(function(){ try { throw false } catch (T) { if (T) console.log("a"); } });
