/// A single backend a `Balancer` can route requests to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backend {
    pub id: u32,
    pub weight: u32,
    pub healthy: bool,
    pub active_connections: u32,
}

impl Backend {
    pub fn new(id: u32, weight: u32) -> Self {
        Backend {
            id,
            weight: weight.max(1),
            healthy: true,
            active_connections: 0,
        }
    }

    pub fn mark_down(&mut self) {
        self.healthy = false;
    }

    pub fn mark_up(&mut self) {
        self.healthy = true;
    }

    pub fn acquire(&mut self) {
        self.active_connections += 1;
    }

    pub fn release(&mut self) {
        self.active_connections = self.active_connections.saturating_sub(1);
    }
}
