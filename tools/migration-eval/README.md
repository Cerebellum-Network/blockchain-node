# migration-eval

Evaluates a candidate runtime's storage migrations against **real chain state**.

## Why this exists

Migration bugs on this chain do not look like crashes. They look like success.

Evaluation round 1 found `ddc-payouts` v2 skipping all 58 Mainnet billing
reports, then v3 reading the same bytes under a different layout that is *also*
exactly 167 bytes — decoding cleanly and writing rewards inflated by 4.2e12×.
try-runtime's own checks passed, because the migration's `pre_upgrade` and
`post_upgrade` both counted undecodable entries and compared `0 == 0`.

A tool that asks "did it error?" ships that. This one asks whether the **values
still mean the same thing**, by capturing storage before and after and comparing
the two decodings.

## Usage

```bash
# WASM_BUILD_WORKSPACE_HINT is not optional. Without it, substrate-wasm-builder
# cannot find the workspace Cargo.lock, generates its own beside the artifact,
# and can pin different git revisions than the workspace — producing a wasm built
# from stale sources while the native build uses current ones.
WASM_BUILD_WORKSPACE_HINT="$PWD" cargo build --release -p cere-runtime

cd tools/migration-eval && npm install
npm run eval -- scenarios/mainnet-release-1.yml
```

The tool prints the git revisions it found in the wasm build's own lock file, and
a scenario can pin them under `runtime.deps` to make a mismatch fatal. Printing
is not checking: a revision that only appears in the log proves nothing about
what the run enforced. Pin what the scenario depends on.

To evaluate a build outside the default target dir — a second worktree, a shared
`CARGO_TARGET_DIR` — set `MIGRATION_EVAL_WASM` rather than editing the scenario's
`wasm:` path. Editing it forks the file, and a scenario that diverges from the one
in git is a scenario nobody has actually run:

```bash
MIGRATION_EVAL_WASM=/path/to/cere_runtime.compact.compressed.wasm \
  npm run eval -- scenarios/mainnet-release-1.yml
```

The override is announced in the output, never silent.

Exit code is non-zero if any assertion fails.

## How it works

1. Fork the target chain with Chopsticks — **without** the candidate runtime, so
   the first capture decodes with pre-upgrade metadata.
2. Snapshot the storage items named under `capture:`.
3. Write the candidate wasm to `:code`. `on_runtime_upgrade` fires on the next
   block, exactly as it would from a real `set_code`.
4. Drive blocks until `MultiBlockMigrations::Cursor` clears, recording per-block
   weight and migration events. Capped by `maxBlocks`, so a migration that never
   terminates fails loudly instead of hanging.
5. Reconnect, so metadata is now post-upgrade, and snapshot again.
6. Evaluate the assertions across the two snapshots.

Steps 1 and 5 are the point: the same bytes decoded under two type universes is
what makes a misaligned-layout migration visible.

## Scenarios are specifications

A scenario states what a release **must** do, and is written from evaluation
findings *before* the fix exists. `expect: fails` marks an assertion that is
known to fail today, so an unexpected failure is distinguishable from a known
one.

**Remove `expect: fails` the moment the fix lands.** Left behind, it reclassifies
a future regression as already-known and drops it out of the unexpected count —
the scenario goes quiet exactly when it should shout. It is scaffolding for a
known-broken window, not a permanent annotation.

If a fix cannot satisfy an assertion and the assertion is changed instead, that
shows up as its own diff. Treat it as a question, not a detail.

## Assertions

| Form | Meaning |
|---|---|
| `count: {equals: N}` | exactly N entries after |
| `rawCount: {equals: N}` | N keys under the raw `twox128` prefix, bypassing metadata |
| `count: {unchanged: true}` | same count before and after |
| `sum: {field: f, unchanged: true}` | Σf identical before and after |
| `sum: {field: f, equalsBefore: {item, field}}` | Σf after equals Σ of another item before — for renamed storage |
| `weight: {peakBlock: "<100%"}` | no block exceeds the limit |

`because:` is required prose explaining what the assertion protects. It is
printed on failure, so a red run explains itself.

**Use `rawCount:` when a migration renames a storage item.** The old item is
absent from post-upgrade metadata, so a metadata-driven `count:` cannot tell
"the prefix is empty" from "the item no longer exists" — it reports `n/a` either
way, and the assertion can never pass however the migration behaves. `rawCount:`
pages `state_getKeysPaged` over `twox128(pallet) ++ twox128(item)` and sees data
orphaned under a prefix the runtime has stopped declaring. That is the shape of
the `v4_mbm` copy-without-remove bug: 535 entries left somewhere the runtime no
longer looks.

## Two Chopsticks requirements, both non-obvious

- **`allowUnresolvedImports: true`** — the runtime imports
  `ext_ddc_dac_close_session_version_1` from `ddc-dac-host`. Chopsticks provides
  only the standard host-function set, so without this it fails at
  instantiation. Unresolved imports then trap only if *called*; migrations touch
  storage rather than DAC sessions, so this is safe. **A migration that calls a
  `ddc-dac-host` function cannot be evaluated here** — use try-runtime.
- **`prefetch:`** — lazy per-key fetching against a ~1s-per-call endpoint stalls
  block building past any sane RPC timeout.

## Runtime pinning

`runtime.sha256` makes the tool refuse to run against an artifact the scenario
was not written for. This is not theoretical: during development a scenario was
run against a stale wasm and the unchanged result was briefly read as a fix
failing to work. Pin it.

**`sha256` is not sufficient on its own.** It proves you tested the artifact you
named — not that the artifact was built from the sources you think.
substrate-wasm-builder runs a nested cargo build with its own `Cargo.lock`
written beside the wasm; if it cannot find the workspace lock it generates one
independently. That happened here: the workspace resolved
`ddc-payouts#8178000c` while the wasm was built from `#b8988623`, so a verified
fix appeared not to work and the runtime hash was byte-identical across the
change — correctly, because it *was* the same wasm.

So the tool reads that lock and reports the revisions compiled into the artifact.
Pin them under `runtime.deps` when a scenario depends on a specific fix being
present.

## Limits

- Chopsticks mocks consensus, so wall-clock timing means nothing. Block *counts*
  are meaningful; block *durations* are not.
- Migration `pre_upgrade`/`post_upgrade` hooks do not run — those need
  try-runtime. Given round 1, that is a small loss.
- `sum:` walks decoded JSON for a named field, matching either the Rust
  snake_case name or the camelCase that `toJSON()` emits. A field that resolves
  on neither side is reported as **FIELD NOT FOUND — assertion proved nothing**,
  never as a value comparison. That distinction matters: an assertion that
  silently reads nothing and passes is the same failure mode as a migration's
  `ensure!(0 == 0)`, which is what this tool exists to catch.
- `state_getRuntimeVersion` reports the version Chopsticks resolved for the fork,
  not the one the head is executing, so it does not move across a `:code` write.
  The tool therefore reads the applied version from `System::LastRuntimeUpgrade`,
  which `frame_executive` writes while applying the upgrade, and asserts that it
  advanced. This is not cosmetic: `frame_executive` runs `on_runtime_upgrade`
  only when the runtime's `spec_version` differs from that value, so a release
  that forgets the bump is a silent no-op on a real chain — every migration
  skipped, no error anywhere. It is the one check that catches that.
