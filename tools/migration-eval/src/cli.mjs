#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { parse } from 'yaml';
import { checkRuntime, startFork } from './fork.mjs';
import { connect, capture, drive, evaluate } from './engine.mjs';

const scenarioPath = resolve(process.argv[2] ?? 'scenarios/mainnet-release-1.yml');
const scenario = parse(readFileSync(scenarioPath, 'utf8'));

console.log(`\n  scenario  ${scenario.name}`);
const rt = checkRuntime(scenarioPath, scenario.runtime);
console.log(`  runtime   ${rt.wasm.split('/').slice(-1)[0]}  sha256=${rt.sha.slice(0, 16)}…  ${rt.bytes.length} bytes`);
console.log(`  fork      ${scenario.fork.endpoint} @ ${scenario.fork.at}`);
for (const [crate, d] of Object.entries(rt.deps ?? {}))
  console.log(`  dep       ${crate.padEnd(18)} ${d.rev.slice(0, 8)}  (${d.branch})`);
console.log('');

const { proc, url } = await startFork(scenario.fork);
let api = await connect(url);
try {
  const specBefore = (await api.rpc.state.getRuntimeVersion()).specVersion.toNumber();
  const head = (await api.rpc.chain.getHeader()).number.toNumber();
  console.log(`  forked at #${head}, spec ${specBefore}`);

  // Capture with PRE-upgrade metadata.
  const before = await capture(api, scenario.capture ?? []);
  for (const [k, v] of Object.entries(before)) console.log(`    before  ${k.padEnd(38)} ${v.missing ? 'absent' : v.count}`);

  // Apply the candidate runtime by writing `:code`. on_runtime_upgrade fires on
  // the next block, exactly as it would from a real set_code.
  console.log(`\n  applying candidate runtime via :code …`);
  // `:code` = 0x3a636f6465. Chopsticks takes raw keys as [key, value] pairs.
  await api.rpc('dev_setStorage', [['0x3a636f6465', '0x' + rt.bytes.toString('hex')]]);

  const run = await drive(api, scenario.drive ?? {});
  for (const b of run.blocks)
    console.log(`    #${b.number}  ${String(b.pct.toFixed(2)).padStart(7)}%  ${b.events.join(', ') || (b.cursorActive ? 'cursor active' : '')}`);
  if (!run.terminated) console.log(`    !! hit maxBlocks — migrations did not terminate`);

  // Reconnect so polkadot-js picks up POST-upgrade metadata.
  await api.disconnect();
  api = await connect(url);
  const specAfter = (await api.rpc.state.getRuntimeVersion()).specVersion.toNumber();
  console.log(`\n  spec ${specBefore} -> ${specAfter},  blocks driven ${run.blocks.length}`);

  const after = await capture(api, [...(scenario.capture ?? []), ...(scenario.assert?.storage ?? []).map((s) => s.item)]);
  const results = evaluate(scenario, before, after, run);

  console.log(`\n  ${'-'.repeat(66)}`);
  let failed = 0, surprises = 0;
  for (const r of results) {
    const known = r.expected === 'fails';
    if (!r.ok && r.severity === 'fail') { failed++; if (!known) surprises++; }
    const mark = r.ok ? 'PASS' : r.severity === 'warn' ? 'WARN' : known ? 'FAIL (known)' : 'FAIL';
    console.log(`  ${mark.padEnd(13)} ${(r.item ? r.item + ' ' : '') + r.name}`);
    console.log(`                ${r.detail}`);
    if (!r.ok && r.because) console.log(`                why: ${r.because.replace(/\s+/g, ' ')}`);
  }
  console.log(`  ${'-'.repeat(66)}`);
  console.log(`  ${results.filter((r) => r.ok).length} passed, ${failed} failed` +
              (failed ? `  (${failed - surprises} known, ${surprises} unexpected)` : ''));
  process.exitCode = failed ? 1 : 0;
} finally {
  await api?.disconnect().catch(() => {});
  proc.kill();
}
