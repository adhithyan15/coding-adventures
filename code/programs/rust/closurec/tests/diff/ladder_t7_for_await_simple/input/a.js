async function f(s){ for await (const v of s) { console.log(v); } }
console.log(typeof f);
