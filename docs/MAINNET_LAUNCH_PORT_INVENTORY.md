# `mainnet-launch` — Port Inventory

Author: Krishna Singh
Date: 2026-08-12
Purpose: establish what has to move from `origin/staging` onto `mainnet-launch`
(branched from `master`), in what order, and what must **not** move.

Migrations are deliberately out of scope here — they are stubbed during the port
and fixed in a separate workstream. See
`docs/MAINNET_STORAGE_MIGRATION_DRYRUN.md`.

---

## 1. Why this is a manual port, not a merge

The two branches are not "staging is ahead of master". They are two long-lived
lines that diverged and both moved a long way:

| | Value |
|---|---|
| Merge base | `ffd39b4c` — *ClusterGov params on-chain params (#419)* |
| Commits on `master` not in `staging` | **201** |
| Commits on `staging` not in `master` | **1,058** |
| File delta | 133 files, +11,997 / −11,435 |

A trial `git merge origin/staging` onto a branch off `master` produces **72
conflicted files**, including files both sides rewrote independently. The
biggest cause: **both branches migrated to `polkadot-sdk = "=2512.3.2"`
separately**, so the runtime, node and every pallet `Cargo.toml` were rewritten
on both sides toward the same destination by different routes.

Resolving 72 conflicts by hand produces a tree nobody can review and where a
wrong "take theirs" is invisible. Porting deliberately, in slices, is slower but
each slice is reviewable and buildable.

---

## 2. What must NOT move

`mainnet-launch` starts from `master`, so this work is preserved by default.
Listing it because a careless "take staging's version" would silently drop it.

| Master-only work | Why it must survive |
|---|---|
| `feat(hyperbridge): allow Technical Committee alongside Root for ismp origins` | Governance capability not present on staging |
| `feat(hyperbridge): bump to latest 2512.x + swap to pallet-hyper-fungible-token` | Master's Hyperbridge line is ahead of staging's |
| `fix(hyperbridge): restore Coprocessor parachain IDs` | Bug fix, staging never had the bug |
| `fix(hyperbridge): restore cere-dev CreateOrigin to EnsureSigned` | As above |
| `fix(node): polkadot-sdk umbrella not forwarding rocksdb feature` | Build correctness |
| `runtime: keep transaction_version at 25` | Deliberate Mainnet pin |
| **The pallet index map** | The whole point — see §4 |

`runtime/cere/src/hyperbridge_ismp.rs` and `runtime/cere/src/weights/pallet_ismp.rs`
were **added independently on both sides** (add/add conflicts). Master's versions
win; do not port staging's.

---

## 2b. Branch topology and PR plan

```
master
  └── mainnet-launch                     integration branch, long-lived
        ├── feat/port-pallets            PR 1 — building tree, migrations present but UNWIRED
        └── feat/storage-migrations      PR 2 — wire + fix migrations (branch after PR 1 merges)
```

`mainnet-launch` starts as `master`, so Mainnet's pallet indices are preserved by
default. Push it early and empty — it is the branch the external crates'
dev-dependencies will eventually target.

### PR 1 must contain slices A + B + C together

They cannot be separated. Swapping `primitives/` for the external
`ddc-primitives` changes the types every DDC pallet is written against, so the
pallet bodies must move in the same change; the runtime references those
pallets, so its wiring must move too. Any two of the three alone leaves a tree
that does not build.

Slices D (node / chain spec) and E (contract) *can* be separate PRs, but are
small enough to fold in.

Structure PR 1 as **ordered commits** — dependency swap, then pallets, then
runtime — so review can proceed commit by commit even though the PR is large.

### On "no storage migration code" in PR 1

Two readings, and they differ a lot in cost:

| Approach | Cost |
|---|---|
| **Bring the migration files in, leave them unwired** (recommended) | Zero extra work. This is exactly staging's state — `Unreleased` and `UnreleasedMultiblock` are defined but referenced nowhere, and `type Migrations = ()`. |
| Omit the migration files entirely | Requires editing every pallet's `lib.rs` to drop `pub mod migrations;`, then re-adding it in PR 2. Larger diff, diverges from staging, no benefit. |

Recommend the first. PR 1 ends with migration code present and provably inert;
PR 2 wires it and fixes it. The "no migrations" property is enforced by
`type Migrations = ()`, not by the absence of files.

### Ordering unlock for `ddc-payouts`

`ddc-payouts`' PR is **held**. Its dev-dependency `pallet-ddc-customers` points
at a blockchain-node branch that pulls `ddc-primitives?branch=staging`, which
collides with its own `branch = "mainnet-launch"` (see §2c).

It unblocks as soon as **`feat/port-pallets` is pushed** — that branch carries
the ported `ddc-customers` compiling against `ddc-primitives#mainnet-launch`, so
pointing the dev-dep there resolves the collision without waiting for a merge.

**Before either PR merges**, move that dev-dep from `feat/port-pallets` to
`mainnet-launch`, so it does not end up referencing a deleted feature branch.

---

## 2c. Hazard: mixed branch labels produce duplicate crates

Cargo keys a git dependency by **URL *and* branch label**. Two references to the
same commit under different labels resolve to two distinct crates:

```
git+…/ddc-primitives.git?branch=staging#9aaac1ae
git+…/ddc-primitives.git?branch=mainnet-launch#9aaac1ae     <- same commit, different crate
```

Traits from one do not satisfy bounds expecting the other, producing `E0277`
errors that do not name the real cause. This was hit for real in `ddc-payouts`.

`[patch]` cannot fix it — cargo rejects patching a source with itself.

**Therefore: every external crate reference in the runtime graph must point at
`mainnet-launch`, with no stragglers.** Verify before trusting a build:

```
grep -rn 'branch = "staging"' Cargo.toml
grep -c 'ddc-primitives.git?branch' Cargo.lock     # expect exactly one distinct source
```

---

## 3. What must move, by slice

Slices are ordered by dependency. Each should build and be a separate PR into
`mainnet-launch`.

### Two distinct operations — do not conflate them

| Operation | What changes | Applies to |
|---|---|---|
| **Swap** | dependency *source* | `primitives/` → external `ddc-primitives`; `pallets/ddc-payouts/` deleted → external crate; `ddc-verification`, `ddc-api`, `ddc-dac-host` added as new external deps |
| **Port** | *code only*, location unchanged | the five DDC pallets that stay in-tree |

Staging extracted only **two** pallets. The rest stay in this repo:

| In-tree on staging (`path =`) | External on staging (`git =`) |
|---|---|
| `pallet-ddc-clusters` | `ddc-primitives` |
| `pallet-ddc-clusters-gov` | `ddc-api` |
| `pallet-ddc-customers` | `ddc-dac-host` |
| `pallet-ddc-nodes` | `pallet-ddc-verification` |
| `pallet-ddc-staking` | `pallet-ddc-payouts` |

Verified: no `Cerebellum-Network/ddc-{customers,clusters,nodes,staking,clusters-gov}`
repositories exist. Those five are ported in place, not swapped.

### Slice A — dependency swap *(blocks everything else)*

The structural change. Staging moved four things out of this repo:

| Removed from repo | Replaced by external crate |
|---|---|
| `primitives/` (10 files) | `ddc-primitives` — `Cerebellum-Network/ddc-primitives` |
| `pallets/ddc-payouts/` (6 files) | `pallet-ddc-payouts` — `Cerebellum-Network/ddc-payouts` |
| — | `pallet-ddc-verification` — `Cerebellum-Network/ddc-verification` |
| — | `ddc-api`, `ddc-dac-host` |

**Dependency order.** These crates depend on each other, which fixes the order
their branches and PRs must land in:

```
ddc-primitives  →  ddc-api  →  ddc-dac-host  →  ddc-verification  →  ddc-payouts
```

Each crate pins its siblings by **branch name**, so every `mainnet-launch`
branch must repoint its siblings from `branch = "staging"` to
`branch = "mainnet-launch"` — otherwise a Mainnet build silently resolves its
dependencies to Testnet's code.

**A sixth dependency: `ddc-proto`.** `ddc-api` carries
`third_party/ddc-proto` as a **git submodule**, not a cargo dependency, so it
does not appear in any `Cargo.toml`. It needs no `mainnet-launch` branch —
submodules pin by commit SHA already. But it must be cloned with
`--recurse-submodules` or the `ddc-api` build fails in `protoc` with a missing
`signature.proto`.

**Build verification.** Do not review or merge any of these branches without a
green `cargo check --locked --all-features` from a **standalone clone**.
Checkouts placed inside `blockchain-node/` are treated as workspace members and
cargo refuses to run in them.

Also deleted: `pallets/ddc-metrics-offchain-worker` test data (2 files).

Every pallet imports from `primitives`, so this slice necessarily includes the
mechanical import rewrite across `chainbridge`, `origins`,
`pool-withdrawal-fix` (2–13 lines each — import-only) and the DDC pallets.

**All five crates now have `mainnet-launch` branches.** Each repoints its own
siblings, so the chain resolves consistently. blockchain-node must point at
`branch = "mainnet-launch"` for all five — see the §2c hazard.

| Crate | `mainnet-launch` at | PR |
|---|---|---|
| `ddc-primitives` | `9aaac1ae` (= staging) | ddc-primitives#40 |
| `ddc-api` | `c9a38fa0` | ddc-api#48 |
| `ddc-dac-host` | `980548f1` | ddc-dac-host#21 |
| `ddc-verification` | `616ce832` | ddc-verification#72 |
| `ddc-payouts` | `66ff5f0` + repoint | **held** — see §2b |

`ddc-api` additionally carries `third_party/ddc-proto` as a git submodule; clone
with `--recurse-submodules`.

**Dependency pinning — deferred.** These branches track each other by name, not
by commit. Known gap to close before Mainnet: branch tracking means the runtime
silently changes whenever someone pushes, so the runtime that ships would not be
the runtime that was evaluated.

### Slice B — DDC pallet bodies

Code only. Migration modules stubbed, not ported.

| Pallet | Size | Notes |
|---|---|---|
| `ddc-customers` | 9 files, +2,842 / −558 | Largest. `migration.rs` → `migrations.rs` rename; adds `benchmarking_customer_deposit.wasm` |
| `ddc-clusters` | 11 files, +1,991 / −705 | `ClusterProtocolParams` 10 → 14 fields, new `AccountId` generic |
| `ddc-nodes` | 9 files, +775 / −122 | `migrations.rs` is new — file does not exist on master |
| `fee-handler` | 6 files, +434 / −105 | Gains tests, mock, benchmarking, weights |
| `ddc-clusters-gov` | 6 files, +301 / −196 | |
| `ddc-staking` | 8 files, +247 / −133 | |

### Slice C — runtime wiring

`runtime/cere` (5 files, +494 / −327) and `runtime/common` (1 file, +8 / −6).

- Config impls for `DdcVerification` and the reshaped DDC pallets
- `SessionKeys` gains `ddc_verification` (and `OldSessionKeys` + `transform_session_keys`)
- **Pallet index map: master's, with `DdcVerification` appended at 54** (§4)
- **`spec_version` = 73159** — master's 73158 + 1. The Mainnet line continues
  from what is live; staging's 80011 is a separate lineage and is not adopted.
- `type Migrations` left unwired with a comment pointing at the migration PR

`runtime/cere-dev` mirrors `runtime/cere` and can follow in the same slice.

### Slice D — node and chain spec

`node/service` (4 files, +300 / −21), `node/cli` (3 files), `node/client`,
`node/rpc`. Mostly the new external crates' RPC/OCW wiring.

### Slice E — contract

`contracts/customer-deposit/` (4 files) — new top-level directory, also a
workspace member in `Cargo.toml`. Easy to lose; verify it lands.

### Slice F — CI and tooling *(optional, last)*

`.github/workflows` (4 modified, 4 added, 2 deleted), `Dockerfile`,
`scripts/`, `.secrets.baseline`, `github-checks.sh`. No runtime impact — defer
unless CI blocks the earlier slices.

---

## 4. The pallet index map

The single deliberate divergence from staging. `mainnet-launch` keeps **master's
numbering** and appends `DdcVerification` at the end:

```
40 ConvictionVoting      ← master's, unchanged
41 Referenda             ← unchanged
42 Origins               ← unchanged
43 Whitelist             ← unchanged
44 TechComm              ← unchanged
45 DdcClustersGov        ← unchanged
46 Ismp                  ← unchanged
47 IsmpGrandpa           ← unchanged
48 (unused)              ← keep the gap
49 TokenGateway          ← unchanged
50 FeeHandler            ← unchanged
51 MultiBlockMigrations  ← unchanged
52 DelegatedStaking      ← unchanged
53 PoolWithdrawalFix     ← unchanged
54 DdcVerification       ← NEW, appended
```

Staging inserts `DdcVerification` at 40 and shifts everything below it, which
breaks stored values that encode a pallet index — `RuntimeHoldReason` in
`Balances::Holds` (24 accounts, 17.16M CERE), `RuntimeCall` in
`TechComm::ProposalOf`, `OriginCaller` in `Scheduler::Agenda`. Appending avoids
all three without writing a migration.

Staging also swaps `FeeHandler` (50 → 52) and `MultiBlockMigrations` (51 → 51).
No stored value references those two today, but preserving master's order costs
nothing and keeps call encoding stable.

**Verify the final map against live Mainnet metadata, index by index — not by
eye.**

---

## 5. Open decisions

| Decision | Blocks | Notes |
|---|---|---|
| `transaction_version` | Slice C | Master pins 25 deliberately. Appending `DdcVerification` at 54 leaves existing call indices untouched, but the DDC pallet bodies change — if any existing extrinsic's arguments change, this must bump. Verify against live Mainnet metadata rather than assuming. |
| Pin external crates to commits | Before Mainnet | Deferred; port tracks `branch = "staging"` for now (§3, Slice A) |
| Does Mainnet want `ddc-metrics-offchain-worker` removed? | Slice A | Staging deletes its test data; pallet is not in either runtime but holds 1 orphan storage key on Mainnet |
| `v4_mbm` / `v5_mbm` version range | Migration PR | Not this workstream, but needed before the follow-up |

---

## 6. Suggested sequence

```
mainnet-launch (from master)
  ├── A: dependency swap + import rewrite       ← must land first
  ├── B: DDC pallet bodies (migrations stubbed)
  ├── C: runtime wiring + index map
  ├── D: node / chain spec
  ├── E: contracts/customer-deposit
  └── F: CI / tooling (optional)
        └── then: migration fixes, separate workstream
```

**Definition of done for the port:** `mainnet-launch` builds, the pallet index
map matches Mainnet's live metadata plus `DdcVerification` at 54, and a
try-runtime dry run against a Mainnet fork reproduces the *known* migration
failures and nothing new. That last check is what proves the port did not change
behaviour on its own.
