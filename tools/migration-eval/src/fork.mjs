import { spawn } from 'node:child_process';
import { writeFileSync, readFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

/** The tool's own directory — chopsticks must run here to resolve its package,
 *  and its working files belong here (gitignored), not next to the scenario. */
const toolDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');

/** Verify the candidate wasm is the artifact the scenario expects. */
export function checkRuntime(scenarioPath, runtime) {
  const wasm = resolve(dirname(scenarioPath), runtime.wasm);
  if (!existsSync(wasm)) throw new Error(`runtime wasm not found: ${wasm}\nBuild it first: cargo build --release -p cere-runtime`);
  const bytes = readFileSync(wasm);
  const sha = createHash('sha256').update(bytes).digest('hex');
  if (runtime.sha256 && runtime.sha256 !== sha) {
    throw new Error(`runtime sha256 mismatch\n  scenario expects ${runtime.sha256}\n  artifact is       ${sha}\nRefusing to evaluate a runtime the scenario was not written for.`);
  }
  return { wasm, sha, bytes };
}

/**
 * Start Chopsticks WITHOUT the wasm override, so the first capture sees
 * pre-upgrade metadata. The candidate runtime is applied later, by writing
 * `:code`, which is what makes on_runtime_upgrade fire on the next block.
 */
export async function startFork(fork, port = 8011) {
  const cfg = resolve(toolDir, '.chopsticks.yml');
  writeFileSync(cfg, [
    `endpoint: ${fork.endpoint}`,
    fork.at && fork.at !== 'latest' ? `block: ${fork.at}` : null,
    'mock-signature-host: true',
    `allow-unresolved-imports: ${fork.allowUnresolvedImports ? 'true' : 'false'}`,
    `rpc-timeout: ${fork.rpcTimeout ?? 600000}`,
    `port: ${port}`,
    `db: ${resolve(toolDir, '.chopsticks.sqlite')}`,
    fork.prefetch?.length ? 'prefetch-storages:' : null,
    ...(fork.prefetch ?? []).map((p) => `  - ${p}`),
  ].filter(Boolean).join('\n') + '\n');

  const proc = spawn('npx', ['chopsticks', '--config', cfg], { cwd: toolDir, stdio: ['ignore', 'pipe', 'pipe'] });
  let log = '';
  const ready = new Promise((res, rej) => {
    const t = setTimeout(() => rej(new Error(`chopsticks did not become ready in 300s\n${tail(log)}`)), 300000);
    const scan = (d) => { log += d; if (String(d).includes('listening on')) { clearTimeout(t); res(); } };
    proc.stdout.on('data', scan); proc.stderr.on('data', scan);
    proc.on('exit', (c) => { clearTimeout(t); rej(new Error(`chopsticks exited with ${c}\n${tail(log)}`)); });
  });
  await ready;
  return { proc, url: `ws://localhost:${port}` };
}

const tail = (s, n = 25) => s.trim().split('\n').slice(-n).map((l) => '    | ' + l).join('\n');
