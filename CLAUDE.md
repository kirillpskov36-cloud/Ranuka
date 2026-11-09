# Working on Ranuka

Solana bonding-curve launchpad. The product is the on-chain program plus a
bot-facing SDK; the TUI is a second client on top of the same SDK.

Read `docs/architecture.md` first, then `docs/decisions.md`.

## How we work

- The owner decides architecture and product questions and does not review code
  line by line. Explain changes in terms of architecture and behavior, in Russian.
- Code, identifiers, code comments and commit messages: English.
  Docs under `docs/` and the README: Russian.
- Any decision that changes architecture, economics or the public interface
  gets an entry in `docs/decisions.md`, and `docs/architecture.md` is updated
  in the same change.
- Never start work that needs a product decision by guessing: ask the owner.

## Commands

```sh
./scripts/check.sh        # fmt + clippy (-D warnings) + tests; must pass before every commit
./scripts/check-env.sh    # verify local toolchain
cargo test -p ranuka-core # one crate
```

## Layout and dependency rules

- `crates/ranuka-core` — pricing math. No dependencies, `no_std`, no `unsafe`.
- `programs/ranuka-program` — Anchor program. All pricing via `ranuka-core`.
- `crates/ranuka-sdk` — bot API. Offline modules need no features; networked
  modules sit behind the `rpc` feature.
- `crates/ranuka-cli` — `ranuka` binary. Public SDK only.

Dependencies point one way: `core` <- `program`, `core` <- `sdk` <- `cli`.
The program and the SDK never depend on each other's code.

## Code rules

- Money math: integers only, `u64` at boundaries, `u128` intermediates,
  checked arithmetic, rounding in favor of the pool. `ranuka-core` and the
  program deny `clippy::arithmetic_side_effects`; an `allow` needs a comment
  proving the operation cannot overflow.
- Functions in `ranuka-core` return `Result<_, CoreError>` and never panic.
- Every program instruction ships with success tests and negative tests
  (wrong account, wrong signer, wrong state).
- Each invariant listed in `docs/architecture.md` section 8 has a test.
- After mainnet: existing instructions, account layouts and error codes are
  frozen; new behavior goes into new instructions. Error codes are append-only.
- External dependencies are declared in the root `[workspace.dependencies]`.
- Never commit keypairs or `.env` files.
