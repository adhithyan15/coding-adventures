// Canonical, Object.is-faithful value representation. Mirrors the Rust
// literal evaluator in tests/conformance.rs. Numbers use V8's String() (which
// closurec's format_js_number matches), with -0/NaN/Inf tagged explicitly.
function canon(v){
  if (typeof v === 'number'){
    if (Number.isNaN(v)) return 'n:NaN';
    if (v === 0) return Object.is(v,-0) ? 'n:-0' : 'n:0';
    if (!Number.isFinite(v)) return v>0 ? 'n:Infinity' : 'n:-Infinity';
    return 'n:' + String(v);
  }
  if (typeof v === 'string') return 's:' + v;
  if (typeof v === 'boolean') return 'b:' + v;
  if (v === null) return 'null';
  if (Array.isArray(v)) return '[' + v.map(x => x===undefined ? 'hole' : canon(x)).join(',') + ']';
  if (typeof v === 'object') return '{' + Object.entries(v).map(([k,val]) => 'k:'+k+'='+canon(val)).join(',') + '}';
  if (v === undefined) return 'undef';
  return '?';
}
const sources = [
  "3","1.5","-1","100000000000000000000","1e21",
  '"abcd".slice(1,3)','"ab".repeat(3)','"HELLO".toLowerCase()','"abc".length',
  '"a,b,c".split(",")','"abcabc".indexOf("c")','"x".padStart(3,"0")',
  'Math.max(1,2,3)','Math.min(5,2,8)','Math.max(-5,-1)',
  'String.fromCharCode(65,66)','Number.isInteger(5)','Number.isSafeInteger(10)',
  'Array.isArray([])','Array.of(1,2,3)','Object.keys({})',
  'Object.entries({a:1,b:2})','Object.fromEntries([["a",1],["b",2]])',
  'isNaN("x")','isFinite(3)','Boolean(0)','String(42)','Number("7")',
  // edge / known-divergence:
  "-0",
];
for (const s of sources){
  let val, ok=true;
  try { val = eval('('+s+')'); } catch(e){ ok=false; }
  console.log(JSON.stringify(s) + "\t" + (ok ? canon(val) : 'ERR'));
}
