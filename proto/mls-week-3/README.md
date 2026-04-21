# ISY MLS Prototype — Week 3 Gate

**Status:** throwaway — do not merge to `main`.

Proves MLS end-to-end encryption works against a live Convex deployment.

See authoritative design: [../../docs/MLS_PROTOTYPE.md](../../docs/MLS_PROTOTYPE.md).
See execution spec: [../../docs/superpowers/specs/2026-04-21-mls-prototype-execution-design.md](../../docs/superpowers/specs/2026-04-21-mls-prototype-execution-design.md).

## Quick start

```bash
# one-time: install deps, auth Convex, create isy-dev project
pnpm install              # inside proto/mls-week-3/
npx convex dev            # interactive — first run authenticates, later runs hot-push schema changes

# build the Rust CLI
cargo build --release

# run the demo (see MLS_PROTOTYPE.md § Demo script for full sequence)
./target/release/isy-proto register --uin 600000001 --db /tmp/alice.sqlite
```

## Outcome

- **Pass:** tag `proto-mls-passing-v1`, archive demo recording at `docs/proofs/mls-week-3-pass.mp4` on `main`, proceed to weeks 4-5 planning
- **Fail:** write `POSTMORTEM.md` in this directory, stop, re-plan
