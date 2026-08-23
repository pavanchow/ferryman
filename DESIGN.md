# Design

Ferryman separates the decision from the delivery. It never opens a socket. It takes a set of backend states and a strategy, and answers one question: which backend gets the next request. Everything else, health checks over the wire, actual proxying, connection pooling, is left to whatever wraps this library.

## Backend model

`src/backend.rs` defines `Backend`:

- `id: u32` - identifies the backend.
- `weight: u32` - relative share for weighted round-robin, clamped to at least 1.
- `healthy: bool` - whether the backend can currently be selected.
- `active_connections: u32` - live connection count, used by least-connections.

`mark_down` / `mark_up` flip health. `acquire` / `release` adjust the active connection count, with `release` saturating at zero so it can never underflow.

## Balancer

`src/balancer.rs` defines `Balancer`, which owns a `Vec<Backend>` plus per-strategy state, and a `Strategy` enum selecting the algorithm. `next()` is the single entry point and never panics: it returns `Result<u32, NoBackendAvailable>`, where the id is the chosen backend and the error is a typed unit struct meaning nothing healthy was available (empty pool or every backend down).

### Round-robin

A cursor (`rr_cursor`) tracks where the last pick left off. Each call scans forward from the cursor, wrapping with modulo, and returns the first healthy backend it finds, advancing the cursor past it. Skipping unhealthy backends falls out naturally from the scan; no separate filtering step is needed.

### Weighted round-robin

Implements the smooth weighted round-robin algorithm (the same shape nginx uses). Each backend keeps a running `current_weight` (`wrr_current_weight`, indexed alongside `backends`). On every pick:

1. Every healthy backend's current weight increases by its own weight.
2. The backend with the highest current weight wins.
3. The winner's current weight decreases by the sum of all healthy weights.

Over a full cycle of `total_weight` picks, this produces each backend exactly as many times as its weight, spread out rather than clustered, which is what "weighted" should mean in practice. Unhealthy backends are excluded from the weight sum and never accumulate, so they neither win nor skew the distribution while down.

### Least-connections

A direct `min_by_key` over healthy backends, keyed on `(active_connections, id)` so ties resolve deterministically to the lower id instead of whatever order the standard library happens to walk in.

### Random

`src/rng.rs` implements `Xorshift64`, a small xorshift64* generator seeded with a `u64`. It is not cryptographic, it does not need to be. It exists so `Strategy::Random(seed)` is fully reproducible: the same seed against the same backend states always produces the same sequence of picks, which is what makes the strategy testable at all. `next_below(n)` reduces a raw 64-bit output mod `n` to pick uniformly among the currently healthy backend ids.

## Why a typed error instead of panicking or returning an Option

`NoBackendAvailable` carries the one real degenerate case in this domain: nothing is up. Callers building an actual proxy need to distinguish "route this request" from "there is nowhere to route it" and respond accordingly (503, retry, alert), so it is a named type with a `Display` impl and `std::error::Error`, not a silent `None` or a crash.

## What is deliberately out of scope

No sockets, no health-check probing over HTTP or TCP, no config file format, no persistence. Health and connection state are pushed in by the caller (`mark_down`, `mark_up`, `acquire`, `release`). That boundary is what keeps the strategies pure functions of state and lets the test suite assert exact orderings and distributions without a network in the loop.
