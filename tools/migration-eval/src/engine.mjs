import { ApiPromise, WsProvider } from '@polkadot/api';

export const connect = (url) =>
  ApiPromise.create({ provider: new WsProvider(url, 2500, {}, 900000), noInitWarn: true });

const split = (spec) => { const [p, i] = spec.split('::'); return [lower(p), lower(i)]; };
const lower = (s) => s.charAt(0).toLowerCase() + s.slice(1);

/**
 * Snapshot named storage items, decoded with whatever metadata the connection
 * currently has. Called once before the upgrade (pre-metadata types) and once
 * after (post-metadata types) -- comparing the two is how a migration that
 * reinterprets old bytes under a new layout becomes visible.
 */
export async function capture(api, items) {
  const out = {};
  for (const spec of items) {
    const [p, i] = split(spec);
    if (!api.query[p]?.[i]) { out[spec] = { missing: true, entries: [] }; continue; }
    const entries = await api.query[p][i].entries();
    out[spec] = {
      count: entries.length,
      entries: entries.map(([k, v]) => ({ key: k.toHex(), value: v.toJSON() })),
    };
  }
  return out;
}

/** Sum a numeric field across captured entries. Returns null if absent. */
export function sumField(cap, field) {
  if (!cap || cap.missing) return null;
  let total = 0n, seen = 0;
  for (const { value } of cap.entries) {
    const v = pluck(value, field);
    if (v === undefined || v === null) continue;
    total += BigInt(typeof v === 'string' ? v : Math.trunc(Number(v)));
    seen++;
  }
  return seen ? total : null;
}

const camel = (s) => s.replace(/_([a-z])/g, (_, c) => c.toUpperCase());

function pluck(obj, field) {
  if (obj == null || typeof obj !== 'object') return undefined;
  for (const k of [field, camel(field)]) if (k in obj) return obj[k];
  for (const v of Object.values(obj)) { const r = pluck(v, field); if (r !== undefined) return r; }
  return undefined;
}

/** Drive blocks until the multi-block migration cursor clears. */
export async function drive(api, { maxBlocks = 400 } = {}) {
  const maxBlock = api.consts.system.blockWeights.maxBlock.refTime.toBigInt();
  const blocks = [];
  let done = false;
  for (let i = 0; i < maxBlocks && !done; i++) {
    await api.rpc('dev_newBlock', { count: 1 });
    const hdr = await api.rpc.chain.getHeader();
    const w = await api.query.system.blockWeight();
    const used = w.mandatory.refTime.toBigInt() + w.normal.refTime.toBigInt() + w.operational.refTime.toBigInt();
    const cursor = await api.query.multiBlockMigrations?.cursor?.();
    const events = (await api.query.system.events())
      .filter((r) => r.event.section === 'multiBlockMigrations')
      .map((r) => r.event.method);
    blocks.push({ number: hdr.number.toNumber(), used, pct: Number(used * 10000n / maxBlock) / 100, cursorActive: cursor?.isSome ?? false, events });
    if (i > 0 && cursor && cursor.isNone) done = true;
  }
  return { maxBlock, blocks, terminated: done };
}

/** Evaluate the scenario's assertions against before/after captures. */
export function evaluate(scenario, before, after, run) {
  const results = [];
  const a = scenario.assert ?? {};

  if (a.weight) {
    const peak = run.blocks.reduce((m, b) => (b.used > m ? b.used : m), 0n);
    const pct = Number(peak * 10000n / run.maxBlock) / 100;
    for (const [k, limit] of Object.entries(a.weight)) {
      const bound = Number(String(limit).replace(/[<>%]/g, ''));
      const ok = pct < bound;
      results.push({
        kind: 'weight', name: `${k} ${limit}`, ok,
        detail: `peak ${peak} = ${pct.toFixed(2)}% of maxBlock`,
        severity: k === 'warnAbove' ? 'warn' : 'fail',
      });
    }
  }

  for (const s of a.storage ?? []) {
    const bef = before[s.item], aft = after[s.item];
    if (s.count?.equals !== undefined)
      results.push(check(s, `count == ${s.count.equals}`, aft?.count === s.count.equals, `count=${aft?.count ?? 'n/a'}`));
    if (s.count?.unchanged)
      results.push(check(s, 'count unchanged', bef?.count === aft?.count, `${bef?.count ?? 'n/a'} -> ${aft?.count ?? 'n/a'}`));
    if (s.sum?.unchanged) {
      const b = sumField(bef, s.sum.field), f = sumField(aft, s.sum.field);
      results.push(sumResult(s, `sum(${s.sum.field}) unchanged`, b, f, b !== null && b === f));
    }
    if (s.sum?.equalsBefore) {
      const b = sumField(before[s.sum.equalsBefore.item], s.sum.equalsBefore.field);
      const f = sumField(aft, s.sum.field);
      results.push(sumResult(s, `sum(${s.sum.field}) preserved from ${s.sum.equalsBefore.item}`, b, f, b !== null && f !== null && b === f));
    }
  }
  return results;
}

/** Distinguish "field not found" from "values differ". A null sum means the
 *  scenario names a field that does not exist in the decoded value -- the
 *  assertion proved nothing, which must never read as a value comparison. */
function sumResult(s, name, before, after, ok) {
  if (before === null || after === null) {
    const which = [before === null && 'before', after === null && 'after'].filter(Boolean).join(' and ');
    return { kind: 'storage', item: s.item, name, ok: false, severity: 'fail', expected: null,
      detail: `FIELD NOT FOUND in ${which} — assertion proved nothing`,
      because: `scenario names field '${s.sum.field}'; check it exists in the decoded value` };
  }
  const ratio = Number(after * 1000n / before) / 1000;
  return { kind: 'storage', item: s.item, name, ok, severity: 'fail',
    expected: s.expect ?? null, because: s.because?.trim(),
    detail: `${before} -> ${after}${ratio !== 1 ? `  (x${ratio.toExponential(3)})` : ''}` };
}

const check = (s, name, ok, detail) => ({
  kind: 'storage', item: s.item, name, ok, detail,
  because: s.because?.trim(), expected: s.expect ?? null,
  severity: 'fail',
});
