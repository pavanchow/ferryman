use clap::{Parser, Subcommand, ValueEnum};
use ferryman::{Backend, Balancer, Strategy};

#[derive(Parser)]
#[command(name = "ferryman", about = "Load-balancing strategies, without the network plumbing")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Route a batch of synthetic requests and print which backend each one went to.
    Demo {
        #[arg(long, value_enum, default_value = "round-robin")]
        algo: Algo,
        #[arg(long, default_value_t = 3)]
        backends: u32,
        #[arg(long, default_value_t = 12)]
        requests: u32,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Algo {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    Random,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Demo { algo, backends, requests, seed } => run_demo(algo, backends, requests, seed),
    }
}

fn run_demo(algo: Algo, backend_count: u32, requests: u32, seed: u64) {
    if backend_count == 0 {
        eprintln!("need at least one backend");
        std::process::exit(1);
    }

    let pool: Vec<Backend> = (0..backend_count)
        .map(|i| Backend::new(i, i + 1))
        .collect();

    let strategy = match algo {
        Algo::RoundRobin => Strategy::RoundRobin,
        Algo::WeightedRoundRobin => Strategy::WeightedRoundRobin,
        Algo::LeastConnections => Strategy::LeastConnections,
        Algo::Random => Strategy::Random(seed),
    };

    println!("strategy: {}", strategy);
    println!("backends: {}", backend_count);
    println!();

    let mut balancer = Balancer::new(pool, strategy);
    let mut tally = vec![0u32; backend_count as usize];

    for req in 1..=requests {
        match balancer.next() {
            Ok(id) => {
                balancer.acquire(id);
                tally[id as usize] += 1;
                println!("request {:>3} -> backend {}", req, id);
                if matches!(strategy, Strategy::LeastConnections) {
                    balancer.release(id);
                }
            }
            Err(err) => {
                println!("request {:>3} -> {}", req, err);
            }
        }
    }

    println!();
    println!("tally:");
    for (id, count) in tally.iter().enumerate() {
        println!("  backend {} : {}", id, count);
    }
}
