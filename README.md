# Ferryman

**A load balancer in Rust, stripped down to the part that decides.**

Ferryman is the request-distribution logic and health tracking that sit at the core of a load balancer, with no sockets or network plumbing attached. Given a set of backends and their current state, it tells you which one gets the next request. That is the whole job, and it is deterministic and testable in isolation.

## Strategies

- **Round-robin** - walks the backend list in order, wrapping around, skipping unhealthy backends as it goes.
- **Weighted round-robin** - smooth weighted selection. Each pick accumulates a current weight per backend and takes the highest, so a full cycle matches each backend's weight exactly.
- **Least-connections** - picks the healthy backend with the fewest active connections right now, ties broken by id.
- **Random (seeded)** - a small xorshift64* PRNG picks uniformly among healthy backends. Seed it and the sequence reproduces exactly.

## Health and connections

A backend can be marked down or up. Unhealthy backends are never selected by any strategy, and marking one healthy again puts it straight back in rotation. Connection accounting is explicit: `acquire` increments a backend's active count, `release` decrements it, and least-connections reads that count directly. If every backend is unhealthy, or the pool is empty, `next()` returns a typed `NoBackendAvailable` error instead of panicking.

## Usage

As a library:

```rust
use ferryman::{Backend, Balancer, Strategy};

let backends = vec![Backend::new(0, 1), Backend::new(1, 1), Backend::new(2, 1)];
let mut balancer = Balancer::new(backends, Strategy::RoundRobin);

let id = balancer.next()?; // Ok(0)
balancer.mark_down(1);
let id = balancer.next()?; // skips backend 1
```

As a CLI:

```
cargo run -- demo --algo round-robin --backends 3 --requests 12
cargo run -- demo --algo weighted-round-robin --backends 4 --requests 20
cargo run -- demo --algo least-connections --backends 3 --requests 15
cargo run -- demo --algo random --backends 3 --requests 15 --seed 42
```

Each run prints which backend every request went to, plus a final tally.

## Testing

```
cargo test
```

Covers round-robin ordering, weighted round-robin distribution over a full cycle, least-connections selection under acquire/release, health toggling, the all-unhealthy and empty-pool error paths, and reproducibility of the seeded random strategy.

## Try it in the browser

`docs/index.html` is a self-contained JavaScript port of the same strategies: toggle backends unhealthy, switch strategy, send requests, and watch routing reroute live.

By Pavan Nallamothu.
