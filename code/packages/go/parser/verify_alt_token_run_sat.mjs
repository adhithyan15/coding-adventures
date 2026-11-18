#!/usr/bin/env node
// Optional finite SAT cross-check for the parser's leading token-alternative
// matcher. The production Go package has no Node or SAT-solver dependency.
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { CnfBuilder, negateLit } from '../../typescript/test-bootstrap/dist/sat.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const payloadPrefix = 'KILN_G1_SAT_OBS_JSON: ';
const names = ['', 'NUMBER', 'STRING', 'NAME'];
const expectedPrefix = ['NUMBER', 'STRING', 'NAME'];
const payloadKeys = ['rows', 'version'];
const rowKeys = ['enum', 'failed', 'id', 'matched', 'newline', 'position', 'type_name'];

function fail(message) {
  throw new Error(message);
}

function readObservation(mode, rowID) {
  const env = { ...process.env, KILN_G1_SAT_MODE: mode };
  delete env.KILN_G1_SAT_ROW;
  if (rowID !== undefined) env.KILN_G1_SAT_ROW = String(rowID);
  const stdout = execFileSync(
    'go', ['test', '-count=1', '-v', '-run', '^TestLeadingTokenRunBoundedDomain$', '.'],
    { cwd: here, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] },
  );
  const payloads = stdout.split(/\r?\n/).filter((line) => line.startsWith(payloadPrefix));
  if (payloads.length !== 1) fail(`expected one Go observation payload, got ${payloads.length}`);
  let decoded;
  try {
    decoded = JSON.parse(payloads[0].slice(payloadPrefix.length));
  } catch (error) {
    fail(`invalid Go observation JSON: ${error}`);
  }
  if (decoded.version !== 1 || !Array.isArray(decoded.rows)) fail('unexpected Go payload version or rows');
  if (JSON.stringify(Object.keys(decoded).sort()) !== JSON.stringify(payloadKeys)) {
    fail(`unexpected Go payload fields ${Object.keys(decoded).join(',')}`);
  }
  return decoded.rows;
}

function validateRows(rows, complete) {
  if (complete && rows.length !== 32) fail(`expected 32 Go rows, got ${rows.length}`);
  const byID = new Map();
  for (const row of rows) {
    if (JSON.stringify(Object.keys(row).sort()) !== JSON.stringify(rowKeys)) {
      fail(`unexpected row fields at id ${row.id}`);
    }
    if (!Number.isInteger(row.id) || row.id < 0 || row.id >= 32 || byID.has(row.id)) {
      fail(`invalid or duplicate row id ${row.id}`);
    }
    if (!names.includes(row.type_name) || typeof row.enum !== 'string' || typeof row.newline !== 'boolean') {
      fail(`invalid row input at id ${row.id}`);
    }
    if (typeof row.matched !== 'boolean' || !Number.isInteger(row.position) || !Array.isArray(row.failed) ||
        row.failed.some((value) => typeof value !== 'string')) {
      fail(`invalid row output at id ${row.id}`);
    }
    const expected = expectedPrefix.slice(0, row.failed.length);
    if (JSON.stringify(row.failed) !== JSON.stringify(expected)) {
      fail(`non-contiguous or reordered failure prefix at id ${row.id}: ${JSON.stringify(row.failed)}`);
    }
    if (row.matched !== (row.failed.length < expectedPrefix.length)) {
      fail(`success does not agree with failed prefix at id ${row.id}`);
    }
    const expectedPos = row.matched ? 1 + Number(row.newline) : 0;
    if (row.position !== expectedPos) fail(`wrong final position at id ${row.id}`);
    byID.set(row.id, row);
  }
  if (complete) {
    for (let id = 0; id < 32; id += 1) {
      if (!byID.has(id)) fail(`missing Go observation row ${id}`);
    }
    const enumNames = [...new Set(rows.map((row) => row.enum))];
    if (enumNames.length !== 4) fail(`expected four Go enum constructors, got ${enumNames.join(',')}`);
    if (JSON.stringify(enumNames) !== JSON.stringify(['Number', 'String', 'Name', 'Plus'])) {
      fail(`unexpected Go enum constructors/order ${enumNames.join(',')}`);
    }
    for (let ni = 0; ni < names.length; ni += 1) {
      for (let ei = 0; ei < enumNames.length; ei += 1) {
        for (let nl = 0; nl < 2; nl += 1) {
          const id = (ni * enumNames.length + ei) * 2 + nl;
          const row = byID.get(id);
          if (row.type_name !== names[ni] || row.enum !== enumNames[ei] || row.newline !== (nl === 1)) {
            fail(`Go row order/domain mismatch at id ${id}`);
          }
        }
      }
    }
  }
  return byID;
}

function exactlyOne(cnf, literals) {
  cnf.addClause(literals);
  for (let i = 0; i < literals.length; i += 1) {
    for (let j = i + 1; j < literals.length; j += 1) {
      cnf.addClause([negateLit(literals[i]), negateLit(literals[j])]);
    }
  }
}

function or(cnf, literals) {
  if (literals.length === 0) return cnf.falseLit();
  return literals.slice(1).reduce((left, right) => cnf.or(left, right), literals[0]);
}

function and(cnf, literals) {
  if (literals.length === 0) return cnf.trueLit();
  return literals.slice(1).reduce((left, right) => cnf.and(left, right), literals[0]);
}

function constrainWhen(cnf, condition, output, value) {
  cnf.addClause([negateLit(condition), value ? output : negateLit(output)]);
}

function normalizedPredicate(cnf, typeName, enums, enumNames) {
  const empty = typeName[0];
  const namedNumber = typeName[names.indexOf('NUMBER')];
  const namedString = typeName[names.indexOf('STRING')];
  const namedName = typeName[names.indexOf('NAME')];
  const enumNumber = enums[enumNames.indexOf('Number')];
  const enumString = enums[enumNames.indexOf('String')];
  const enumName = enums[enumNames.indexOf('Name')];
  const normNumber = cnf.or(namedNumber, cnf.and(empty, enumNumber));
  const normString = cnf.or(namedString, cnf.and(empty, enumString));
  const normName = cnf.or(namedName, cnf.and(empty, enumName));
  return { normNumber, normString, normName, enumNumber, enumString };
}

function buildMiter(rows, enumNames, mutateNumberFallback) {
  const cnf = new CnfBuilder();
  const typeName = names.map(() => cnf.newLit());
  const enums = enumNames.map(() => cnf.newLit());
  const newline = cnf.newLit();
  exactlyOne(cnf, typeName);
  exactlyOne(cnf, enums);

  const observed = Array.from({ length: 6 }, () => cnf.newLit());
  const [obsMatch, obsFailNumber, obsFailString, obsFailName, obsPosOne, obsPosTwo] = observed;
  const model = Array.from({ length: 6 }, () => cnf.newLit());
  const [modelMatch, modelFailNumber, modelFailString, modelFailName, modelPosOne, modelPosTwo] = model;

  const outputBits = (row) => [
    row.matched,
    row.failed.includes('NUMBER'),
    row.failed.includes('STRING'),
    row.failed.includes('NAME'),
    row.position === 1,
    row.position === 2,
  ];
  const rowByInput = new Map(rows.map((row) => [`${row.type_name}\u0000${row.enum}\u0000${row.newline}`, row]));
  for (let ni = 0; ni < names.length; ni += 1) {
    for (let ei = 0; ei < enumNames.length; ei += 1) {
      for (let nl = 0; nl < 2; nl += 1) {
        const row = rowByInput.get(`${names[ni]}\u0000${enumNames[ei]}\u0000${nl === 1}`);
        if (!row) fail(`missing Go row for ${names[ni]}/${enumNames[ei]}/newline=${nl === 1}`);
        const prefix = nl === 1 ? newline : negateLit(newline);
        const condition = and(cnf, [typeName[ni], enums[ei], prefix]);
        for (const [i, value] of outputBits(row).entries()) {
          constrainWhen(cnf, condition, observed[i], value);
        }
      }
    }
  }

  const p = normalizedPredicate(cnf, typeName, enums, enumNames);
  // The independent ordered-leaf model: NAME has no legacy enum fallback;
  // NUMBER/STRING retain their old enum fallback. The mutant removes only
  // NUMBER's fallback, exposing a later-alternative priority discrepancy.
  const matchNumber = mutateNumberFallback ? p.normNumber : cnf.or(p.normNumber, p.enumNumber);
  const matchString = cnf.or(p.normString, p.enumString);
  const matchName = p.normName;
  const failNumber = cnf.not(matchNumber);
  const failString = cnf.and(failNumber, cnf.not(matchString));
  const failName = cnf.and(failString, cnf.not(matchName));
  const matched = or(cnf, [matchNumber, matchString, matchName]);
  const posOne = cnf.and(matched, cnf.not(newline));
  const posTwo = cnf.and(matched, newline);
  const modeled = [matched, failNumber, failString, failName, posOne, posTwo];

  const mismatches = observed.map((value, i) => cnf.xor(value, modeled[i]));
  const mismatch = or(cnf, mismatches);
  const solver = cnf.getSolver();
  return { cnf, solver, mismatch, typeName, enums, newline };
}

const rows = readObservation('all');
const byID = validateRows(rows, true);
const enumNames = [...new Set(rows.map((row) => row.enum))];

const base = buildMiter(rows, enumNames, false);
if (base.solver.solve() !== 'sat') fail('base Go-observation/ordered-predicate model is not SAT');
base.cnf.addClause([base.mismatch]);
if (base.solver.solve() !== 'unsat') fail('the correctly encoded mismatch miter is not UNSAT');

// Rebuild the same observation relation and mismatch circuit, changing only
// the independent NUMBER enum-fallback predicate.
const mutant = buildMiter(rows, enumNames, true);
mutant.cnf.addClause([mutant.mismatch]);
if (mutant.solver.solve() !== 'sat') fail('the enum-fallback/priority mutant did not produce a SAT witness');
const model = mutant.solver.getModel();
const selected = (literals) => {
  const indices = literals.flatMap((lit, index) => (model[lit >> 1] ? [index] : []));
  if (indices.length !== 1) fail(`expected one selected input, got ${indices.length}`);
  return indices[0];
};
const typeIndex = selected(mutant.typeName);
const enumIndex = selected(mutant.enums);
const hasNewline = model[mutant.newline >> 1];
const rowID = (typeIndex * enumNames.length + enumIndex) * 2 + Number(hasNewline);
const replayRows = readObservation('replay', rowID);
const replay = validateRows(replayRows, false);
if (replay.size !== 1 || !replay.has(rowID)) fail(`Go replay did not return only row ${rowID}`);
const actual = replay.get(rowID);
if (JSON.stringify(actual) !== JSON.stringify(byID.get(rowID))) fail(`Go replay changed observation for row ${rowID}`);

const normalized = actual.type_name || ({ Number: 'NUMBER', String: 'STRING', Name: 'NAME', Plus: 'PLUS' })[actual.enum];
const mutantP0 = normalized === 'NUMBER';
const p1 = normalized === 'STRING' || actual.enum === 'String';
const p2 = normalized === 'NAME';
const mutantFailed = [];
if (!mutantP0) mutantFailed.push('NUMBER');
if (!mutantP0 && !p1) mutantFailed.push('STRING');
if (!mutantP0 && !p1 && !p2) mutantFailed.push('NAME');
const mutantMatched = mutantP0 || p1 || p2;
const mutantPosition = mutantMatched ? 1 + Number(actual.newline) : 0;
if (mutantMatched === actual.matched && mutantPosition === actual.position &&
    JSON.stringify(mutantFailed) === JSON.stringify(actual.failed)) {
  fail(`decoded SAT witness row ${rowID} does not differ under the mutant`);
}

console.log(`bounded Go SAT check passed: 32 rows; base SAT; correct mismatch UNSAT; enum-fallback mutant SAT; replay row ${rowID} (${actual.type_name || '<empty>'}/${actual.enum}, newline=${actual.newline})`);
