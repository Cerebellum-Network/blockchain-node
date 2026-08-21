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
  // MIGRATION_EVAL_WASM points a scenario at a build outside the default target
  // dir (a second worktree, a shared CARGO_TARGET_DIR) without editing — and so
  // forking — the scenario file. Announced, never silent: evaluating a different
  // artifact than the scenario names must be visible in the log.
  const override = process.env.MIGRATION_EVAL_WASM;
  const wasm = override ? resolve(override) : resolve(dirname(scenarioPath), runtime.wasm);
  if (override) console.log(`  NOTE      wasm overridden by MIGRATION_EVAL_WASM, not the scenario's path`);
  if (!existsSync(wasm)) throw new Error(`runtime wasm not found: ${wasm}\nBuild it first: cargo build --release -p cere-runtime`);
  const bytes = readFileSync(wasm);
  const sha = createHash('sha256').update(bytes).digest('hex');
  if (runtime.sha256 && runtime.sha256 !== sha) {
    throw new Error(`runtime sha256 mismatch\n  scenario expects ${runtime.sha256}\n  artifact is       ${sha}\nRefusing to evaluate a runtime the scenario was not written for.`);
  }

  const deps = readWasmDeps(wasm);
  for (const [crate, want] of Object.entries(runtime.deps ?? {})) {
    const got = deps[crate];
    if (!got) throw new Error(`scenario pins ${crate}, but it is not in the wasm build's lock file`);
    if (!got.rev.startsWith(want))
      throw new Error(
        `${crate} revision mismatch\n  scenario expects ${want}\n  wasm was built from ${got.rev} (branch ${got.branch})\n` +
        `Rebuild with WASM_BUILD_WORKSPACE_HINT set to the workspace root — wasm-builder\n` +
        `resolves dependencies through its own lock file, which can diverge from the workspace's.`);
  }
  return { wasm, sha, bytes, deps };
}

/**
 * Read the git revisions actually compiled into the wasm.
 *
 * substrate-wasm-builder runs a nested cargo build with its OWN Cargo.lock,
 * written beside the artifact. If it cannot find the workspace lock it generates
 * one independently, which can pin different revisions than the workspace —
 * silently producing a wasm built from stale sources while the native build uses
 * current ones. `sha256` cannot catch that: it proves you tested the artifact you
 * named, not that the artifact came from the sources you think.
 */
export function readWasmDeps(wasmPath) {
  const lock = resolve(dirname(wasmPath), 'Cargo.lock');
  if (!existsSync(lock)) return {};
  const out = {};
  for (const m of readFileSync(lock, 'utf8').matchAll(/source = "git\+[^"]*\/([a-z0-9-]+)\.git\?branch=([^#"]+)#([0-9a-f]+)"/g))
    out[m[1]] = { branch: m[2], rev: m[3] };
  return out;
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
