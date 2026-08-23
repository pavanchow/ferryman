use crate::backend::Backend;
use crate::rng::Xorshift64;

/// Which distribution strategy a `Balancer` uses to pick the next backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    /// Seeded so a run can be reproduced exactly.
    Random(u64),
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Strategy::RoundRobin => write!(f, "round-robin"),
            Strategy::WeightedRoundRobin => write!(f, "weighted-round-robin"),
            Strategy::LeastConnections => write!(f, "least-connections"),
            Strategy::Random(_) => write!(f, "random"),
        }
    }
}

/// Every backend was unhealthy (or the pool is empty) when a pick was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoBackendAvailable;

impl std::fmt::Display for NoBackendAvailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no healthy backend available")
    }
}

impl std::error::Error for NoBackendAvailable {}

pub struct Balancer {
    backends: Vec<Backend>,
    strategy: Strategy,
    rr_cursor: usize,
    wrr_current_weight: Vec<i64>,
    rng: Xorshift64,
}

impl Balancer {
    pub fn new(backends: Vec<Backend>, strategy: Strategy) -> Self {
        let n = backends.len();
        let seed = match strategy {
            Strategy::Random(s) => s,
            _ => 0,
        };
        Balancer {
            backends,
            strategy,
            rr_cursor: 0,
            wrr_current_weight: vec![0; n],
            rng: Xorshift64::new(seed),
        }
    }

    pub fn backends(&self) -> &[Backend] {
        &self.backends
    }

    pub fn backends_mut(&mut self) -> &mut [Backend] {
        &mut self.backends
    }

    fn index_of(&self, id: u32) -> Option<usize> {
        self.backends.iter().position(|b| b.id == id)
    }

    pub fn mark_down(&mut self, id: u32) {
        if let Some(i) = self.index_of(id) {
            self.backends[i].mark_down();
        }
    }

    pub fn mark_up(&mut self, id: u32) {
        if let Some(i) = self.index_of(id) {
            self.backends[i].mark_up();
        }
    }

    pub fn acquire(&mut self, id: u32) {
        if let Some(i) = self.index_of(id) {
            self.backends[i].acquire();
        }
    }

    pub fn release(&mut self, id: u32) {
        if let Some(i) = self.index_of(id) {
            self.backends[i].release();
        }
    }

    /// Pick the next backend id according to the configured strategy,
    /// skipping unhealthy backends. Never panics; returns a typed error
    /// when nothing healthy is left to pick.
    pub fn next(&mut self) -> Result<u32, NoBackendAvailable> {
        match self.strategy {
            Strategy::RoundRobin => self.next_round_robin(),
            Strategy::WeightedRoundRobin => self.next_weighted_round_robin(),
            Strategy::LeastConnections => self.next_least_connections(),
            Strategy::Random(_) => self.next_random(),
        }
    }

    fn next_round_robin(&mut self) -> Result<u32, NoBackendAvailable> {
        let n = self.backends.len();
        if n == 0 {
            return Err(NoBackendAvailable);
        }
        for step in 0..n {
            let i = (self.rr_cursor + step) % n;
            if self.backends[i].healthy {
                self.rr_cursor = (i + 1) % n;
                return Ok(self.backends[i].id);
            }
        }
        Err(NoBackendAvailable)
    }

    fn next_weighted_round_robin(&mut self) -> Result<u32, NoBackendAvailable> {
        let n = self.backends.len();
        if n == 0 {
            return Err(NoBackendAvailable);
        }
        let total_weight: i64 = self
            .backends
            .iter()
            .filter(|b| b.healthy)
            .map(|b| b.weight as i64)
            .sum();
        if total_weight == 0 {
            return Err(NoBackendAvailable);
        }

        let mut best: Option<usize> = None;
        for i in 0..n {
            if !self.backends[i].healthy {
                continue;
            }
            self.wrr_current_weight[i] += self.backends[i].weight as i64;
            if best.is_none() || self.wrr_current_weight[i] > self.wrr_current_weight[best.unwrap()] {
                best = Some(i);
            }
        }

        let winner = best.ok_or(NoBackendAvailable)?;
        self.wrr_current_weight[winner] -= total_weight;
        Ok(self.backends[winner].id)
    }

    fn next_least_connections(&mut self) -> Result<u32, NoBackendAvailable> {
        self.backends
            .iter()
            .filter(|b| b.healthy)
            .min_by_key(|b| (b.active_connections, b.id))
            .map(|b| b.id)
            .ok_or(NoBackendAvailable)
    }

    fn next_random(&mut self) -> Result<u32, NoBackendAvailable> {
        let healthy_ids: Vec<u32> = self
            .backends
            .iter()
            .filter(|b| b.healthy)
            .map(|b| b.id)
            .collect();
        if healthy_ids.is_empty() {
            return Err(NoBackendAvailable);
        }
        let idx = self.rng.next_below(healthy_ids.len());
        Ok(healthy_ids[idx])
    }
}
