mod constants;
mod create3;
mod gpu_search;
#[cfg(feature = "metal")]
mod test_shader;

use alloy_primitives::{keccak256, Address, B256};
use anyhow::Result;
use clap::Parser;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::task;

use crate::create3::{compute_create2_address, compute_create3_address};
use gpu_search::{GpuBackend, GpuVanitySearch};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(subcommand_required = true)]
enum Args {
    /// Search for vanity CREATE2 addresses
    Create2 {
        /// The deployer address
        deployer: String,

        /// The bytecode hash to use
        bytecode_hash: String,

        /// The prefix to search for
        prefix: String,

        /// Enable CPU mining
        #[arg(long = "cpu.enabled", default_value = "false")]
        cpu_enabled: bool,

        /// Number of CPU threads to use
        #[arg(long = "cpu.threads")]
        cpu_threads: Option<usize>,

        /// Enable GPU mining
        #[arg(long = "gpu.enabled", default_value = "true")]
        gpu_enabled: bool,

        /// Force specific GPU API (cuda or metal)
        #[arg(long = "gpu.api")]
        gpu_api: Option<String>,

        /// Number of GPU threads per block/threadgroup
        #[arg(long = "gpu.threads")]
        gpu_threads: Option<u32>,

        /// Number of blocks/threadgroups
        #[arg(long = "gpu.thread-groups", default_value = "65536")]
        gpu_thread_groups: u32,
    },

    /// Search for vanity CREATE3 addresses
    Create3 {
        /// The deployer address
        deployer: String,

        /// The prefix to search for
        prefix: String,

        /// Optional namespace
        #[arg(long)]
        namespace: Option<String>,

        /// Enable CPU mining
        #[arg(long = "cpu.enabled", default_value = "false")]
        cpu_enabled: bool,

        /// Number of CPU threads to use
        #[arg(long = "cpu.threads")]
        cpu_threads: Option<usize>,

        /// Enable GPU mining
        #[arg(long = "gpu.enabled", default_value = "true")]
        gpu_enabled: bool,

        /// Force specific GPU API (cuda or metal)
        #[arg(long = "gpu.api")]
        gpu_api: Option<String>,

        /// Number of GPU threads per block/threadgroup
        #[arg(long = "gpu.threads")]
        gpu_threads: Option<u32>,

        /// Number of blocks/threadgroups
        #[arg(long = "gpu.thread-groups", default_value = "65536")]
        gpu_thread_groups: u32,
    },
}

fn create_random_salt() -> B256 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();
    let random = rand::random::<u64>().to_string();
    let combined = format!("{}{}", now, random);
    let hash = keccak256(combined.as_bytes()).to_vec();
    B256::from_slice(&hash)
}

struct Stats {
    attempts: Arc<AtomicU64>,
    last_print: Arc<Mutex<Instant>>,
}

impl Stats {
    fn new() -> Self {
        Self {
            attempts: Arc::new(AtomicU64::new(0)),
            last_print: Arc::new(Mutex::new(Instant::now())),
        }
    }

    fn increment(&self) {
        self.attempts.fetch_add(1, Ordering::Relaxed);

        let mut last_print = self.last_print.lock().unwrap();
        if last_print.elapsed() >= Duration::from_secs(1) {
            let attempts = self.attempts.swap(0, Ordering::Relaxed);
            println!("CPU Attempts per second: {}", attempts);
            *last_print = Instant::now();
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args {
        Args::Create2 {
            deployer,
            prefix,
            bytecode_hash,
            cpu_enabled,
            cpu_threads,
            gpu_enabled,
            gpu_api,
            gpu_threads,
            gpu_thread_groups,
        } => {
            // Parse the deployer address and bytecode hash
            let deployer: Address = deployer.parse().expect("Invalid deployer address");
            let prefix = prefix.to_lowercase();
            let bytecode_hash: B256 = bytecode_hash.parse().expect("Invalid bytecode hash");
            let num_threads = cpu_threads.unwrap_or_else(num_cpus::get);

            if cpu_enabled {
                println!(
                    "Starting {} CPU threads to search for vanity CREATE2 address",
                    num_threads
                );
            }
            println!("Deployer: {}", deployer);
            println!("Prefix: {}", prefix);
            println!("Bytecode Hash: {}", bytecode_hash);

            let mut handles = vec![];
            let stats = Arc::new(Stats::new());
            let running = Arc::new(AtomicU64::new(1));

            // Start CPU threads if enabled
            if cpu_enabled {
                for _ in 0..num_threads {
                    let stats = Arc::clone(&stats);
                    let running = Arc::clone(&running);
                    let deployer = deployer;
                    let prefix = prefix.clone();
                    let bytecode_hash = bytecode_hash;

                    let handle = task::spawn(async move {
                        while running.load(Ordering::Relaxed) == 1 {
                            let salt = create_random_salt();
                            let address = compute_create2_address(deployer, salt, bytecode_hash);

                            stats.increment();

                            if format!("{:x}", address).starts_with(&prefix) {
                                println!("\nCPU Found matching address!");
                                println!("Address: {}", address);
                                println!("Salt: {}", salt);
                                running.store(0, Ordering::Relaxed);
                                break;
                            }
                        }
                    });
                    handles.push(handle);
                }
            }

            // Initialize GPU search if enabled
            if gpu_enabled {
                let gpu_search = if let Some(api) = gpu_api {
                    match api.to_lowercase().as_str() {
                        #[cfg(feature = "cuda")]
                        "cuda" => GpuVanitySearch::new_with_backend(GpuBackend::Cuda),
                        #[cfg(feature = "metal")]
                        "metal" => GpuVanitySearch::new_with_backend(GpuBackend::Metal),
                        _ => {
                            let mut valid_apis = Vec::new();
                            #[cfg(feature = "cuda")]
                            valid_apis.push("'cuda'");
                            #[cfg(feature = "metal")]
                            valid_apis.push("'metal'");
                            panic!("Invalid GPU API specified. Use {}", valid_apis.join(" or "));
                        }
                    }
                } else {
                    GpuVanitySearch::new()
                };

                if let Some(gpu) = gpu_search {
                    println!("Using GPU for mining...");
                    while running.load(Ordering::Relaxed) == 1 {
                        if let Some(salt) = gpu.search_create2(
                            &deployer.to_vec(),
                            &prefix,
                            bytecode_hash.as_slice(),
                            create_random_salt().as_slice(),
                            gpu_threads,
                            Some(gpu_thread_groups),
                        ) {
                            let salt = B256::from_slice(&salt);
                            let address = compute_create2_address(deployer, salt, bytecode_hash);

                            // Verify the address matches the prefix
                            let addr_str = format!("{:x}", address);
                            if !addr_str.starts_with(&prefix.to_lowercase()) {
                                panic!(
                                    "GPU found invalid address: expected prefix '{}' but got '{}'",
                                    prefix, addr_str
                                );
                            }

                            println!("\nGPU Found matching address!");
                            println!("Address: {}", address);
                            println!("Salt: {}", salt);
                            running.store(0, Ordering::Relaxed);
                            break;
                        }
                    }
                } else {
                    println!("GPU support not available");
                }
            }

            // Wait for any thread to find a result if CPU mining is enabled
            if cpu_enabled {
                futures::future::join_all(handles).await;
            }
        }
        Args::Create3 {
            deployer,
            prefix,
            namespace,
            cpu_enabled,
            cpu_threads,
            gpu_enabled,
            gpu_api,
            gpu_threads,
            gpu_thread_groups,
        } => {
            // Parse the deployer address
            let deployer: Address = deployer.parse().expect("Invalid deployer address");
            let prefix = prefix.to_lowercase();
            let namespace = namespace.as_deref().unwrap_or("");
            let num_threads = cpu_threads.unwrap_or_else(num_cpus::get);

            if cpu_enabled {
                println!(
                    "Starting {} CPU threads to search for vanity CREATE3 address",
                    num_threads
                );
            }
            println!("Deployer: {}", deployer);
            println!("Prefix: {}", prefix);
            if !namespace.is_empty() {
                println!("Namespace: {}", namespace);
            }

            let mut handles = vec![];
            let stats = Arc::new(Stats::new());
            let running = Arc::new(AtomicU64::new(1));

            // Start CPU threads if enabled
            if cpu_enabled {
                for _ in 0..num_threads {
                    let stats = Arc::clone(&stats);
                    let running = Arc::clone(&running);
                    let deployer = deployer;
                    let prefix = prefix.clone();
                    let namespace = namespace.to_string();

                    let handle = task::spawn(async move {
                        while running.load(Ordering::Relaxed) == 1 {
                            let salt = create_random_salt();
                            let address = compute_create3_address(deployer, salt, Some(&namespace))
                                .expect("Failed to compute address");

                            stats.increment();

                            if format!("{:x}", address).starts_with(&prefix) {
                                println!("\nCPU Found matching address!");
                                println!("Address: {}", address);
                                println!("Salt: {}", salt);
                                running.store(0, Ordering::Relaxed);
                                break;
                            }
                        }
                    });
                    handles.push(handle);
                }
            }

            // Initialize GPU search if enabled
            if gpu_enabled {
                let gpu_search = if let Some(api) = gpu_api {
                    match api.to_lowercase().as_str() {
                        #[cfg(feature = "cuda")]
                        "cuda" => GpuVanitySearch::new_with_backend(GpuBackend::Cuda),
                        #[cfg(feature = "metal")]
                        "metal" => GpuVanitySearch::new_with_backend(GpuBackend::Metal),
                        _ => {
                            let mut valid_apis = Vec::new();
                            #[cfg(feature = "cuda")]
                            valid_apis.push("'cuda'");
                            #[cfg(feature = "metal")]
                            valid_apis.push("'metal'");
                            panic!("Invalid GPU API specified. Use {}", valid_apis.join(" or "));
                        }
                    }
                } else {
                    GpuVanitySearch::new()
                };

                if let Some(gpu) = gpu_search {
                    println!("Using GPU for mining...");
                    while running.load(Ordering::Relaxed) == 1 {
                        if let Some(salt) = gpu.search_create3(
                            &deployer.to_vec(),
                            &prefix,
                            namespace,
                            create_random_salt().as_slice(),
                            gpu_threads,
                            Some(gpu_thread_groups),
                        ) {
                            let salt = B256::from_slice(&salt);
                            let address = compute_create3_address(deployer, salt, Some(namespace))
                                .expect("Failed to compute address");

                            // Verify the address matches the prefix
                            let addr_str = format!("{:x}", address);
                            if !addr_str.starts_with(&prefix.to_lowercase()) {
                                panic!(
                                    "GPU found invalid address: expected prefix '{}' but got '{}'",
                                    prefix, addr_str
                                );
                            }

                            println!("\nGPU Found matching address!");
                            println!("Address: {}", address);
                            println!("Salt: {}", salt);
                            running.store(0, Ordering::Relaxed);
                            break;
                        }
                    }
                } else {
                    println!("GPU support not available");
                }
            }

            // Wait for any thread to find a result if CPU mining is enabled
            if cpu_enabled {
                futures::future::join_all(handles).await;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_random_salt_generation() {
        // Test that random salts are unique
        let mut salts = HashSet::new();
        for _ in 0..100 {
            let salt = create_random_salt();
            assert!(!salts.contains(&salt), "Generated duplicate salt");
            salts.insert(salt);
        }
    }

    #[test]
    fn test_stats_tracking() {
        let stats = Stats::new();

        // Test initial state
        assert_eq!(stats.attempts.load(Ordering::Relaxed), 0);

        // Test increment
        stats.increment();
        assert_eq!(stats.attempts.load(Ordering::Relaxed), 1);

        // Test multiple increments
        for _ in 0..10 {
            stats.increment();
        }
        assert_eq!(stats.attempts.load(Ordering::Relaxed), 11);

        // Test reset after elapsed time
        std::thread::sleep(Duration::from_secs(1));
        stats.increment();
        assert_eq!(stats.attempts.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_vanity_address_search() {
        // Test with a simple prefix that should be found quickly
        let deployer = "0x343431e9CEb7C19cC8d3eA0EE231bfF82B584910"
            .parse()
            .expect("Invalid test deployer address");
        let prefix = "0";
        let namespace = "";
        let num_threads = 2;

        let stats = Arc::new(Stats::new());
        let running = Arc::new(AtomicU64::new(1));
        let mut handles = vec![];

        let runtime = tokio::runtime::Runtime::new().unwrap();

        for _ in 0..num_threads {
            let stats = Arc::clone(&stats);
            let running = Arc::clone(&running);
            let prefix = prefix.to_string();

            let handle = runtime.spawn(async move {
                let mut found_salt = None;
                while running.load(Ordering::Relaxed) == 1 {
                    let salt = create_random_salt();
                    let address = compute_create3_address(deployer, salt, Some(namespace))
                        .expect("Failed to compute address");

                    stats.increment();

                    if format!("{:x}", address).starts_with(prefix.as_str()) {
                        found_salt = Some(salt);
                        running.store(0, Ordering::Relaxed);
                        break;
                    }
                }
                found_salt
            });

            handles.push(handle);
        }

        let results = runtime.block_on(async { futures::future::join_all(handles).await });
        let found_salt = results.into_iter().find_map(|r| r.unwrap());

        assert!(found_salt.is_some(), "Failed to find matching address");

        // Verify the found salt produces an address with the correct prefix
        if let Some(salt) = found_salt {
            let address = compute_create3_address(deployer, salt, Some(namespace))
                .expect("Failed to compute address");
            assert!(
                format!("{:x}", address).starts_with(prefix),
                "Found address does not match prefix"
            );
        }
    }
}
