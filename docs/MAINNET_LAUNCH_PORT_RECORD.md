# `feat/port-pallets` — Port Record

Author: Krishna Singh
Date: 2026-08-20
Branch: `feat/port-pallets` → `mainnet-launch` (off `master`)
Companion: [`MAINNET_LAUNCH_PORT_INVENTORY.md`](MAINNET_LAUNCH_PORT_INVENTORY.md)

What was taken from `staging`, what was kept from `master`, and why. Migrations
are deliberately **not** wired here — that is a separate PR.

---

## Method

`master` is the default. Nothing arrives from `staging` without a reason, and
nothing arrives by bulk directory copy. The two branches diverged at `ffd39b4c`
— 201 commits on `master`, 1,058 on `staging` — and **both migrated to
polkadot-sdk 2512.3.2 independently**, so `staging` is not simply ahead. Each
branch holds work the other lacks, and a bulk copy loses `master`'s silently,
without producing a conflict.

Scale of the result:

| | Selective | Bulk copy would be |
|---|---|---|
| Files | 91 | 133 |
| Lines | +9,269 / −9,982 | +11,997 / −11,435 |
| `runtime/cere/src/lib.rs` | +276 / −86 | +474 / −304 |
| `runtime/cere-dev/src/lib.rs` | +190 / −36 | +360 / −236 |

Of 93 hunks in `runtime/cere/src/lib.rs`, **49 were pure import-path churn**
(`staging` drops the `polkadot_sdk::` prefix) and were rejected. `cere-dev`: 46
of 85, likewise.

---

## Master-only content preserved

Each of these would have been silently replaced by a bulk copy. The first three
are Mainnet-correctness issues.

| Kept | Staging's value | Why it matters |
|---|---|---|
| `StateMachine::Polkadot(3367)` in `runtime/cere/src/hyperbridge_ismp.rs` | `StateMachine::Kusama(4009)` | The ISMP coprocessor. Staging targets the Gargantua **testnet**; Mainnet's bridge must address Nexus on Polkadot. |
| `EitherOfDiverse<EnsureRoot, EnsureMembers<TechCommCollective, 3>>` for `AdminOrigin` / `RootOrigin` / `CreateOrigin` | `EnsureRoot` only | Technical Committee can act alongside Root on ismp origins. Dropping it removes a governance capability. |
| `#[cfg(feature = "try-runtime")] type DisabledValidators = ()` in `pallet_babe::Config` | absent | Lets `try-runtime` author synthetic blocks despite validator 0 being disabled on Mainnet. Without it the multi-block migration dry run panics — this is what stalled evaluation round 1. |
| `pallet-ismp/runtime-benchmarks`, `ismp-grandpa/runtime-benchmarks` forwards | dropped | Hyperbridge crates are outside the umbrella and need explicit forwarding. |
| `pallet-ddc-clusters-gov/std` feature forward | dropped | Staging drops the forward while keeping the dependency — appears to be a staging bug. |
| `sc-cli = { default-features = true }` in `node/cli/Cargo.toml` | absent | Restores rocksdb, which the umbrella does not forward. |
| `Migrations` tuple (staking v16, session v1) | two `RemovePallet` entries | See "Migrations" below. |
| `node/rpc/Cargo.toml` | comment relabelling only | Nothing of substance. |

---

## Deliberate divergence from both branches

### Pallet index map — `DdcVerification` appended at 54

`staging` inserts it at **40**, shifting every pallet below. Three storage items
encode a pallet index in their SCALE-encoded *values*, and none of them change
storage version, so a version sweep cannot see the damage:

| Storage | Mainnet exposure |
|---|---|
| `Balances::Holds` (`RuntimeHoldReason`) | 24 accounts, **17,164,730.5167 CERE** |
| `TechComm::ProposalOf` (`RuntimeCall`) | 1 proposal, would become un-closeable |
| `Scheduler::Agenda` (`OriginCaller`) | 1 of 3 slots, would silently read empty |

Appending leaves every existing index untouched, so none of those need a
migration. Verified mechanically: **zero** master indices moved.

### `ddc-customers` call indices — `deposit_for` appended at 7

`staging` inserts `deposit_for` at index **3**, shifting four extrinsics down —
so a caller targeting `unlock_deposit` at 3 would instead invoke a *deposit*.
Renumbered to preserve Mainnet's existing encoding:

```
0 create_bucket   1 deposit     2 deposit_extra   3 unlock_deposit
4 withdraw_unlocked_deposit     5 set_bucket_params
6 remove_bucket   7 deposit_for   <- appended
```

`ddc-clusters` needed no such fix — it appends `join_cluster` at 5 correctly.

### Versions

| | Value | Rationale |
|---|---|---|
| workspace `version` | `7.5.0` | Continues master's 7.4.0; staging's 8.0.1 is a separate lineage |
| `spec_version` | `73159` | master's 73158 + 1 |
| `transaction_version` | `26` | **Must** bump: `ddc-customers` gained an extrinsic and `fee-handler` lost one, so call encoding changed |

`cere-dev` carries the same version numbers to stay in step; the number has no
Mainnet meaning there.

---

## Taken from staging

### Swapped — dependency source changed

| Removed from repo | Now |
|---|---|
| `primitives/` (10 files) | external `ddc-primitives` |
| `pallets/ddc-payouts/` (6 files) | external `pallet-ddc-payouts` |
| — | new: `ddc-api`, `ddc-dac-host`, `pallet-ddc-verification` |

All five point at `branch = "mainnet-launch"`, **including the one inside
`contracts/customer-deposit/Cargo.toml`**. Cargo keys a git source by URL *and*
branch, so a single reference left on `staging` resolves the same commit as a
second, distinct crate and breaks trait bounds with errors that do not name the
cause. Verified: no `branch = "staging"` remains in the tracked tree.

`ddc-api` additionally carries `third_party/ddc-proto` as a git submodule —
clone with `--recurse-submodules`.

### Ported in place — code updated, still in-tree

`ddc-clusters`, `ddc-clusters-gov`, `ddc-customers`, `ddc-nodes`, `ddc-staking`,
`fee-handler`, `chainbridge`, `erc20`, `erc721`, `origins`,
`pool-withdrawal-fix`.

No `Cerebellum-Network/ddc-{customers,clusters,nodes,staking,clusters-gov}`
repositories exist — those five are ported, not swapped.

An API-surface diff across all 11 pallets found only one genuine removal
(`fee-handler`, below). Everything else flagged was a rename:
`AccountsLedger`→`CustomerLedger`, `sub_account_id`→`cluster_vault_id`,
`OldCluster`→ versioned `v0/v1/v2::Cluster`.

### Added

`contracts/customer-deposit/` — ink! contract, new workspace member, plus the
`ink` workspace dependency and `CereChainExtension` in both runtimes. The
contract calls `func_id 1` to fetch the `DdcPayouts` pallet account so it can
authorise incoming `charge` calls.

### Runtime and node

- `OldSessionKeys` + `SessionKeys` gaining `ddc_verification`, and
  `transform_session_keys` (`cere` only — staging's `cere-dev` has no transform)
- `DdcVerification` config and its supporting parameter types
- The DDC config region, swapped wholesale as one bounded region
- `ocw_heap_pages` / `--ocw-heap-pages N` in `node/service` — a dynamic
  off-chain-worker heap up to 512 MB, needed because `DdcVerification` runs OCWs
- `default = ["cli", "cere-native", "cere-dev-native"]` in `node/cli`

---

## `fee-handler` — the one genuine feature loss

Staging removes the `fee_distribution_config` extrinsic and its
`FeeDistributionProportionConfig` storage, and renumbers `burn_native_tokens`
from index 2 to 1. **Staging's version was taken**, deliberately.

Why it is not simply a regression to fix:

- Staging's rework is real engineering — it adds a `WeightInfo` associated type
  and benchmarks in place of three hardcoded `weight(10_000)` annotations, plus
  tests, mock and weights files that `master` never had.
- `handle_fee` differs in *behaviour*, not just surface: `master` splits the fee
  between treasury and fee pot per the stored config; `staging` sends the whole
  fee to the fee pot.
- On Mainnet the config **has never been set** (`FeeDistributionProportionConfig`
  reads `None`), and `master` does not wire `FeeHandler` into anything — so its
  splitting logic has never executed in production.
- `staging` **does** wire it, via `pallet_ddc_payouts::Config { type FeeHandler
  = FeeHandler }`. This port therefore makes fee handling **active for the first
  time**.

Where DDC payout fees go is an economic decision, not a porting one. Restoring
master's splitting logic would ship untested behaviour *and* error
`FeeDistributionConfigNotSet` on every payout until governance set a proportion.

**Open question for whoever owns fee handling:** is the configurable
treasury/fee-pot split still wanted? If yes, re-add the extrinsic at index 1
with `burn_native_tokens` back at 2 — a ~40-line change on top of staging's
version, keeping its weights and tests.

---

## Migrations — present but inert

Staging's `migrations` module (`Unreleased`, `UnreleasedMultiblock`) is included
in `runtime/cere` and **wired to nothing** — `migrations::Unreleased` has zero
references. `pallet_migrations::Config::Migrations` is `()`. The "no migrations
run" property is enforced by the type, so a dry run can prove it, rather than by
files being absent.

`Executive`'s `Migrations` tuple keeps master's two versioned FRAME migrations
(staking v16, session v1). Both are self-gating and Mainnet is already at those
versions, so they no-op.

The two `RemovePallet` cleanups master carried (`Hyperbridge`, old
`TokenGateway`) were **dropped**, as master's own comment instructed: they
shipped with 73158, `RemovePallet` is not self-gating, and both prefixes read
empty on Mainnet.

---

## API changes that surfaced as compile errors

Not silent breakage — the compiler named each one:

| Change | Consequence |
|---|---|
| `pallet-pool-withdrawal-fix::Config` gained `RuntimeEvent` | added in both runtimes |
| `pallet-ddc-staking::Config` merged `NodeVisitor` + `NodeCreator` into `NodeManager` | `cere-dev` places this config outside the swapped region, so it needed a separate fix |

---

## Verification

```
cargo check --workspace     # exit 0, no errors
```

Marker checks on `runtime/cere`:

```
StateMachine::Polkadot(3367)                      PRESENT
EitherOfDiverse<…EnsureMembers<TechCommCollective, 3>>  PRESENT
type DisabledValidators = ()                      PRESENT
MigrateV15ToV16                                   PRESENT
pallet-ismp/runtime-benchmarks                    PRESENT
pallet-ddc-clusters-gov/std                       PRESENT
StateMachine::Kusama(4009)                        absent
spec_version: 80011                               absent
```

Pallet index map diffed against `master` programmatically: **no existing index
moved**, `DdcVerification` added at 54.

---

## Not done here

- Migrations remain unwired and unfixed — separate PR, see
  `MAINNET_STORAGE_MIGRATION_DRYRUN.md`
- External crates track branches, not pinned commits
- `ddc-payouts`' PR is held until its `pallet-ddc-customers` dev-dependency is
  repointed at this branch; before either merges it should move to
  `mainnet-launch`
- `ddc-metrics-offchain-worker` left alone by instruction; 1 orphan storage key
  remains on Mainnet under that prefix
