//! Ferryman: the request-distribution strategies and health tracking that
//! sit at the core of a load balancer, isolated from any network plumbing.

mod backend;
mod balancer;
mod rng;

pub use backend::Backend;
pub use balancer::{Balancer, NoBackendAvailable, Strategy};
pub use rng::Xorshift64;

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(n: u32) -> Vec<Backend> {
        (0..n).map(|i| Backend::new(i, 1)).collect()
    }

    #[test]
    fn round_robin_cycles_in_order() {
        let mut b = Balancer::new(pool(3), Strategy::RoundRobin);
        let picks: Vec<u32> = (0..9).map(|_| b.next().unwrap()).collect();
        assert_eq!(picks, vec![0, 1, 2, 0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn weighted_round_robin_respects_weights_over_a_cycle() {
        // weights 4:2:1 -> over a full cycle of 7 picks each backend
        // should appear exactly as many times as its weight.
        let backends = vec![
            Backend::new(0, 4),
            Backend::new(1, 2),
            Backend::new(2, 1),
        ];
        let mut b = Balancer::new(backends, Strategy::WeightedRoundRobin);
        let mut counts = [0u32; 3];
        for _ in 0..7 {
            let id = b.next().unwrap();
            counts[id as usize] += 1;
        }
        assert_eq!(counts, [4, 2, 1]);
    }

    #[test]
    fn least_connections_picks_fewest_active() {
        let mut backends = pool(3);
        backends[0].active_connections = 5;
        backends[1].active_connections = 1;
        backends[2].active_connections = 3;
        let mut b = Balancer::new(backends, Strategy::LeastConnections);
        assert_eq!(b.next().unwrap(), 1);
    }

    #[test]
    fn least_connections_uses_acquire_and_release() {
        let mut b = Balancer::new(pool(2), Strategy::LeastConnections);
        // both start at 0 active connections, id 0 wins ties
        assert_eq!(b.next().unwrap(), 0);
        b.acquire(0);
        b.acquire(0);
        b.acquire(1);
        // backend 1 now has fewer active connections
        assert_eq!(b.next().unwrap(), 1);
        b.release(0);
        b.release(0);
        // backend 0 is back to 0, backend 1 has 1
        assert_eq!(b.next().unwrap(), 0);
    }

    #[test]
    fn unhealthy_backend_is_skipped() {
        let mut b = Balancer::new(pool(3), Strategy::RoundRobin);
        b.mark_down(1);
        let picks: Vec<u32> = (0..4).map(|_| b.next().unwrap()).collect();
        assert_eq!(picks, vec![0, 2, 0, 2]);
    }

    #[test]
    fn marking_healthy_again_brings_it_back() {
        let mut b = Balancer::new(pool(3), Strategy::RoundRobin);
        b.mark_down(1);
        assert_eq!(b.next().unwrap(), 0);
        assert_eq!(b.next().unwrap(), 2);
        b.mark_up(1);
        assert_eq!(b.next().unwrap(), 0);
        assert_eq!(b.next().unwrap(), 1);
    }

    #[test]
    fn all_unhealthy_returns_no_backend_available() {
        let mut backends = pool(3);
        for be in backends.iter_mut() {
            be.mark_down();
        }
        let mut b = Balancer::new(backends, Strategy::RoundRobin);
        assert_eq!(b.next(), Err(NoBackendAvailable));
    }

    #[test]
    fn empty_pool_returns_no_backend_available() {
        let mut b = Balancer::new(Vec::new(), Strategy::LeastConnections);
        assert_eq!(b.next(), Err(NoBackendAvailable));
    }

    #[test]
    fn no_backend_available_does_not_panic_across_strategies() {
        for strategy in [
            Strategy::RoundRobin,
            Strategy::WeightedRoundRobin,
            Strategy::LeastConnections,
            Strategy::Random(7),
        ] {
            let mut backends = pool(2);
            for be in backends.iter_mut() {
                be.mark_down();
            }
            let mut b = Balancer::new(backends, strategy);
            assert_eq!(b.next(), Err(NoBackendAvailable));
        }
    }

    #[test]
    fn random_with_fixed_seed_is_reproducible() {
        let mut a = Balancer::new(pool(5), Strategy::Random(1234));
        let mut b = Balancer::new(pool(5), Strategy::Random(1234));
        let picks_a: Vec<u32> = (0..20).map(|_| a.next().unwrap()).collect();
        let picks_b: Vec<u32> = (0..20).map(|_| b.next().unwrap()).collect();
        assert_eq!(picks_a, picks_b);
    }

    #[test]
    fn random_only_ever_picks_healthy_backends() {
        let mut backends = pool(4);
        backends[2].mark_down();
        let mut b = Balancer::new(backends, Strategy::Random(99));
        for _ in 0..50 {
            assert_ne!(b.next().unwrap(), 2);
        }
    }
}
