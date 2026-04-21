# ISY MLS Prototype — Week 3 Gate

**Status:** throwaway — do not merge to `main`.

Proves MLS end-to-end encryption works against a live Convex deployment.

See authoritative design: [../../docs/MLS_PROTOTYPE.md](../../docs/MLS_PROTOTYPE.md).
See execution spec: [../../docs/superpowers/specs/2026-04-21-mls-prototype-execution-design.md](../../docs/superpowers/specs/2026-04-21-mls-prototype-execution-design.md).

## Quick start

Run all commands from inside `proto/mls-week-3/`. The `--ignore-workspace` flag on `pnpm install` is required: without it, pnpm walks up to the repo-root `pnpm-workspace.yaml` and treats this directory as part of the monorepo, installing nothing locally.

```bash
# one-time: install isolated deps, auth Convex, create isy-dev project
cd proto/mls-week-3
pnpm install --ignore-workspace
npx convex dev            # interactive — first run authenticates, later runs hot-push schema changes

# build the Rust CLI
cargo build --release

# run the demo (see MLS_PROTOTYPE.md § Demo script for full sequence)
./target/release/isy-proto register --uin 600000001 --db /tmp/alice.sqlite
```

## Outcome

- **Pass:** tag `proto-mls-passing-v1`, archive demo recording at `docs/proofs/mls-week-3-pass.mp4` on `main`, proceed to weeks 4-5 planning
- **Fail:** write `POSTMORTEM.md` in this directory, stop, re-plan

## Kill test result (Task 15)

On 2026-04-21, the kill test was executed against the live `isy-dev` Convex deployment. Alice (UIN 600000001) submitted one MLS-encrypted message with the unique plaintext marker `KILL_TEST_MARKER_ABC123_the_quick_brown_fox_jumps_over`. The corresponding row in the Convex `messages` table was inspected via the HTTP `messages:fetchCiphertext` mutation with full project-admin access:

- **Ciphertext byte length:** 199
- **First 32 bytes (hex):** `0001000210ca9ed2e8c631bd1829929b792b1a397f000000000000000301001c` — MLS wire-format header (version `0001`, wire format `0002` = PrivateMessage, group ID `ca9ed2e8c631bd18…`, epoch 3, content type Application). This header is routing metadata, not content.
- **Plaintext marker `KILL_TEST_MARKER`:** not present.
- **Substrings `quick_brown_fox` / `ABC123` / `fox`:** not present.
- **Printable-ASCII runs ≥ 4 chars found in the entire 199-byte payload:** exactly 3, all of 4–5 characters, all random (`'U|vy.'`, `'fd+u'`, `'-eA@R'`).

Success criterion #8 from `docs/MLS_PROTOTYPE.md` — "taking a Convex export of the messages table, with full root-level Convex permissions, produces only ciphertext blobs that no tool can decrypt without the device-side keys" — holds.
