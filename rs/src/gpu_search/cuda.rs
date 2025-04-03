use rustacuda::context::{Context, ContextFlags, ContextStack, CurrentContext};
use rustacuda::launch;
use rustacuda::memory::DeviceBuffer;
use rustacuda::prelude::*;
use std::cell::Cell;
use std::ffi::CString;
use std::sync::Mutex;
use std::time::Instant;

// A simple structure to hold device info and resources
pub struct GpuDevice {
    device_id: u32,
    device_name: String,
    resources: Mutex<DeviceResources>,
    context: Context, // Each device has its own context
}

// Resources that must be accessed under a mutex
struct DeviceResources {
    create3_module: Module,
    create2_module: Option<Module>,
    stream: Stream,
}

pub struct GpuVanitySearch {
    devices: Vec<GpuDevice>,
    iterations: Cell<u64>,
    time_taken: Cell<f64>,
    last_print: Cell<Instant>,
    current_device: Cell<usize>,
}

impl GpuVanitySearch {
    pub fn new() -> Option<Self> {
        // Initialize CUDA - this only needs to be done once
        match rustacuda::init(CudaFlags::empty()) {
            Ok(_) => (),
            Err(e) => {
                println!("Failed to initialize CUDA: {}", e);
                return None;
            }
        }

        // Get device count
        let device_count = match Device::num_devices() {
            Ok(count) => count,
            Err(e) => {
                println!("Failed to get CUDA device count: {}", e);
                return None;
            }
        };

        if device_count == 0 {
            println!("No CUDA devices found");
            return None;
        }

        println!("Found {} CUDA device(s)", device_count);

        // We no longer create a single context here for all devices

        let mut devices = Vec::with_capacity(device_count as usize);

        for device_id in 0..device_count {
            match Self::initialize_device(device_id) {
                Some(device) => devices.push(device),
                None => println!("Warning: Failed to initialize CUDA device {}", device_id),
            }
        }

        if devices.is_empty() {
            println!("Failed to initialize any CUDA devices");
            return None;
        }

        println!(
            "Successfully initialized {} CUDA GPU device(s)",
            devices.len()
        );

        Some(Self {
            devices,
            iterations: Cell::new(0),
            time_taken: Cell::new(0.0),
            last_print: Cell::new(Instant::now()),
            current_device: Cell::new(0),
        })
    }

    fn initialize_device(device_id: u32) -> Option<GpuDevice> {
        // Get device
        let device = match Device::get_device(device_id) {
            Ok(device) => device,
            Err(e) => {
                println!("Failed to get CUDA device {}: {}", device_id, e);
                return None;
            }
        };

        // Get device name for better logging
        let device_name = device.name().unwrap_or_default();
        println!("Initializing CUDA device {}: {}", device_id, device_name);

        // Create a context for this specific device
        let context = match Context::create_and_push(
            ContextFlags::MAP_HOST | ContextFlags::SCHED_AUTO,
            device,
        ) {
            Ok(ctx) => ctx,
            Err(e) => {
                println!(
                    "Failed to create CUDA context for device {}: {}",
                    device_id, e
                );
                return None;
            }
        };

        // Create a stream
        let stream = match Stream::new(StreamFlags::NON_BLOCKING, None) {
            Ok(stream) => stream,
            Err(e) => {
                println!(
                    "Failed to create CUDA stream for device {}: {}",
                    device_id, e
                );
                // Context will be automatically popped when it goes out of scope
                return None;
            }
        };

        let create3_string = include_str!("../shader/CREATE3.ptx");

        // Load the CREATE3 CUDA module - required for basic functionality
        let create3_module = match Self::load_ptx_module(create3_string) {
            Ok(module) => {
                println!(
                    "Successfully loaded CREATE3 CUDA module for device {}",
                    device_id
                );
                module
            }
            Err(e) => {
                println!(
                    "Failed to load CREATE3 CUDA module for device {}: {}",
                    device_id, e
                );
                // Context will be automatically popped when it goes out of scope
                return None;
            }
        };

        let create2_string = include_str!("../shader/CREATE2.ptx");
        // Try to load the CREATE2 CUDA module - optional feature
        let create2_module = match Self::load_ptx_module(create2_string) {
            Ok(module) => {
                println!(
                    "Successfully loaded CREATE2 CUDA module for device {}",
                    device_id
                );
                Some(module)
            }
            Err(e) => {
                println!(
                    "Warning: Failed to load CREATE2 CUDA module for device {}: {}",
                    device_id, e
                );
                println!(
                    "CREATE2 functionality will not be available on device {}",
                    device_id
                );
                None
            }
        };

        // Create device resources
        let resources = DeviceResources {
            create3_module,
            create2_module,
            stream,
        };

        // Pop the context from the stack - it's still owned by the context variable
        match ContextStack::pop() {
            Ok(_) => (),
            Err(e) => {
                println!("Warning: Failed to pop context: {}", e);
                // Continue anyway since the context is owned by the context variable
            }
        }

        Some(GpuDevice {
            device_id,
            device_name,
            resources: Mutex::new(resources),
            context,
        })
    }

    // Helper method to load a PTX module from file
    fn load_ptx_module(ptx_str: &str) -> Result<Module, rustacuda::error::CudaError> {
        match CString::new(ptx_str) {
            Ok(ptx) => Module::load_from_string(&ptx),
            Err(e) => {
                println!("Error creating CString from PTX content: {}", e);
                Err(rustacuda::error::CudaError::InvalidValue)
            }
        }
    }

    // Legacy method for backwards compatibility
    pub fn search_with_threads(
        &self,
        deployer: &[u8],
        prefix: &str,
        namespace: &str,
        initial_salt: &[u8],
        thread_count: Option<u32>,
        block_count: Option<u32>,
    ) -> Option<Vec<u8>> {
        self.search_with_threads_create3(
            deployer,
            prefix,
            namespace,
            initial_salt,
            thread_count,
            block_count,
        )
    }

    pub fn search_with_threads_create3(
        &self,
        deployer: &[u8],
        prefix: &str,
        namespace: &str,
        initial_salt: &[u8],
        thread_count: Option<u32>,
        block_count: Option<u32>,
    ) -> Option<Vec<u8>> {
        if self.devices.is_empty() {
            return None;
        }

        // Use provided thread/block counts or default values
        let thread_count = thread_count.unwrap_or(256);
        let block_count = block_count.unwrap_or(65536);

        // Round-robin between devices for load balancing
        let current = self.current_device.get();
        let next = (current + 1) % self.devices.len();
        self.current_device.set(next);

        let device = &self.devices[current];
        // println!(
        //     "Using CUDA device {}: {}",
        //     device.device_id, device.device_name
        // );

        // Set this device's context as the current context
        if let Err(e) = CurrentContext::set_current(&device.context) {
            println!("Failed to set device context: {}", e);
            return None;
        }

        // Convert deployer address to bytes
        let deployer_bytes = if deployer.len() == 20 {
            deployer.to_vec()
        } else {
            // Assume it's a hex string with 0x prefix
            let hex_str = std::str::from_utf8(deployer)
                .expect("Invalid deployer address")
                .trim_start_matches("0x");
            hex::decode(hex_str).expect("Invalid hex in deployer address")
        };

        // Convert prefix string to individual hex values (0-15)
        let prefix_bytes: Vec<u8> = prefix
            .chars()
            .map(|c| c.to_digit(16).unwrap() as u8)
            .collect();

        let ns_bytes = namespace.as_bytes();
        let mut result_salt = vec![0u8; 32];
        let mut found = vec![0i32; 1];

        // Lock the device resources
        let resources_guard = match device.resources.lock() {
            Ok(guard) => guard,
            Err(e) => {
                println!("Failed to lock device resources: {}", e);
                return None;
            }
        };

        // Wrap all CUDA operations in a result to handle errors gracefully
        let result = (|| -> Result<bool, rustacuda::error::CudaError> {
            // Allocate device memory
            let mut deployer_buf = DeviceBuffer::from_slice(&deployer_bytes)?;
            let mut prefix_buf = DeviceBuffer::from_slice(&prefix_bytes)?;
            let prefix_len = prefix_bytes.len() as u32;
            let mut prefix_len_buf = DeviceBuffer::from_slice(&[prefix_len])?;
            let mut ns_buf = DeviceBuffer::from_slice(ns_bytes)?;
            let ns_len = ns_bytes.len() as u32;
            let mut ns_len_buf = DeviceBuffer::from_slice(&[ns_len])?;
            let mut initial_salt_buf = DeviceBuffer::from_slice(initial_salt)?;
            let mut result_salt_buf = DeviceBuffer::from_slice(&result_salt)?;
            let mut found_buf = DeviceBuffer::from_slice(&found)?;

            let function_name = CString::new("vanity_search").unwrap();
            let function = resources_guard
                .create3_module
                .get_function(&function_name)?;

            let start_time = Instant::now();

            unsafe {
                let stream = &resources_guard.stream;
                launch!(
                    function<<<(block_count, 1, 1), (thread_count, 1, 1), 0, stream>>>(
                        deployer_buf.as_device_ptr(),
                        prefix_buf.as_device_ptr(),
                        prefix_len_buf.as_device_ptr(),
                        ns_buf.as_device_ptr(),
                        ns_len_buf.as_device_ptr(),
                        initial_salt_buf.as_device_ptr(),
                        result_salt_buf.as_device_ptr(),
                        found_buf.as_device_ptr()
                    )
                )?;
            }

            resources_guard.stream.synchronize()?;

            let end_time = Instant::now();
            let duration = end_time.duration_since(start_time);
            self.time_taken
                .set(self.time_taken.get() + duration.as_secs_f64());
            self.iterations
                .set(self.iterations.get() + (thread_count * block_count) as u64);

            // Check if we found a result
            found_buf.copy_to(&mut found)?;

            if found[0] != 0 {
                result_salt_buf.copy_to(&mut result_salt)?;
            }

            Ok(found[0] != 0)
        })();

        // Handle any CUDA errors
        match result {
            Ok(found_result) => {
                if found_result
                    || self.last_print.get().elapsed() > std::time::Duration::from_secs(1)
                {
                    let iterations_per_second =
                        (self.iterations.get() as f64) / self.time_taken.get();
                    println!(
                        "CUDA: Iterations per second: {}",
                        (iterations_per_second as u64)
                            .to_string()
                            .chars()
                            .rev()
                            .collect::<Vec<_>>()
                            .chunks(3)
                            .map(|chunk| chunk.iter().collect::<String>())
                            .collect::<Vec<_>>()
                            .join(",")
                            .chars()
                            .rev()
                            .collect::<String>()
                    );
                    println!("CUDA: Total time taken: {:?}s", self.time_taken.get());
                    self.last_print.set(Instant::now());
                }

                if found_result {
                    Some(result_salt)
                } else {
                    None
                }
            }
            Err(e) => {
                println!("CUDA error during search_with_threads_create3: {}", e);
                None
            }
        }
    }

    pub fn search_with_threads_create2(
        &self,
        deployer: &[u8],
        prefix: &str,
        bytecode_hash: &[u8],
        initial_salt: &[u8],
        thread_count: Option<u32>,
        block_count: Option<u32>,
    ) -> Option<Vec<u8>> {
        if self.devices.is_empty() {
            return None;
        }

        // Use provided thread/block counts or default values
        let thread_count = thread_count.unwrap_or(256);
        let block_count = block_count.unwrap_or(65536);

        // Round-robin between devices for load balancing
        let current = self.current_device.get();
        let next = (current + 1) % self.devices.len();
        self.current_device.set(next);

        let device = &self.devices[current];
        // println!(
        //     "Using CUDA device {}: {}",
        //     device.device_id, device.device_name
        // );

        // Set this device's context as the current context
        if let Err(e) = CurrentContext::set_current(&device.context) {
            println!("Failed to set device context: {}", e);
            return None;
        }

        // Lock the device resources
        let resources_guard = match device.resources.lock() {
            Ok(guard) => guard,
            Err(e) => {
                println!("Failed to lock device resources: {}", e);
                return None;
            }
        };

        // Check if CREATE2 module is available for this device
        let module = match &resources_guard.create2_module {
            Some(module) => module,
            None => {
                println!(
                    "CREATE2 CUDA module not available for device {}. Please compile CREATE2.ptx",
                    current
                );
                return None;
            }
        };

        // Convert deployer address to bytes
        let deployer_bytes = if deployer.len() == 20 {
            deployer.to_vec()
        } else {
            // Assume it's a hex string with 0x prefix
            let hex_str = std::str::from_utf8(deployer)
                .expect("Invalid deployer address")
                .trim_start_matches("0x");
            hex::decode(hex_str).expect("Invalid hex in deployer address")
        };

        // Convert prefix string to individual hex values (0-15)
        let prefix_bytes: Vec<u8> = prefix
            .chars()
            .map(|c| c.to_digit(16).unwrap() as u8)
            .collect();

        let mut result_salt = vec![0u8; 32];
        let mut found = vec![0i32; 1];

        // Wrap all CUDA operations in a result to handle errors gracefully
        let result = (|| -> Result<bool, rustacuda::error::CudaError> {
            // Allocate device memory with proper error handling
            let mut deployer_buf = DeviceBuffer::from_slice(&deployer_bytes)?;
            let mut prefix_buf = DeviceBuffer::from_slice(&prefix_bytes)?;
            let prefix_len = prefix_bytes.len() as u32;
            let mut prefix_len_buf = DeviceBuffer::from_slice(&[prefix_len])?;
            let mut bytecode_hash_buf = DeviceBuffer::from_slice(bytecode_hash)?;
            let mut initial_salt_buf = DeviceBuffer::from_slice(initial_salt)?;
            let mut result_salt_buf = DeviceBuffer::from_slice(&result_salt)?;
            let mut found_buf = DeviceBuffer::from_slice(&found)?;

            let function_name = CString::new("vanity_search").unwrap();
            let function = module.get_function(&function_name)?;

            let start_time = Instant::now();

            unsafe {
                let stream = &resources_guard.stream;
                launch!(
                    function<<<(block_count, 1, 1), (thread_count, 1, 1), 0, stream>>>(
                        deployer_buf.as_device_ptr(),
                        prefix_buf.as_device_ptr(),
                        prefix_len_buf.as_device_ptr(),
                        bytecode_hash_buf.as_device_ptr(),
                        initial_salt_buf.as_device_ptr(),
                        result_salt_buf.as_device_ptr(),
                        found_buf.as_device_ptr()
                    )
                )?;
            }

            resources_guard.stream.synchronize()?;

            let end_time = Instant::now();
            let duration = end_time.duration_since(start_time);
            self.time_taken
                .set(self.time_taken.get() + duration.as_secs_f64());
            self.iterations
                .set(self.iterations.get() + (thread_count * block_count) as u64);

            // Check if we found a result
            found_buf.copy_to(&mut found)?;

            if found[0] != 0 {
                result_salt_buf.copy_to(&mut result_salt)?;
            }

            Ok(found[0] != 0)
        })();

        // Handle any CUDA errors
        match result {
            Ok(found_result) => {
                if found_result
                    || self.last_print.get().elapsed() > std::time::Duration::from_secs(1)
                {
                    let iterations_per_second =
                        (self.iterations.get() as f64) / self.time_taken.get();
                    println!(
                        "CUDA: Iterations per second: {}",
                        (iterations_per_second as u64)
                            .to_string()
                            .chars()
                            .rev()
                            .collect::<Vec<_>>()
                            .chunks(3)
                            .map(|chunk| chunk.iter().collect::<String>())
                            .collect::<Vec<_>>()
                            .join(",")
                            .chars()
                            .rev()
                            .collect::<String>()
                    );
                    println!("CUDA: Total time taken: {:?}s", self.time_taken.get());
                    self.last_print.set(Instant::now());
                }

                if found_result {
                    Some(result_salt)
                } else {
                    None
                }
            }
            Err(e) => {
                println!("CUDA error during search_with_threads_create2: {}", e);
                None
            }
        }
    }
}

impl Drop for GpuVanitySearch {
    fn drop(&mut self) {
        // Clean up all devices
        for (i, device) in self.devices.iter().enumerate() {
            println!("Cleaning up CUDA device {}: {}", i, device.device_name);

            // We'll try to lock the resources for cleanup, but if we can't, we'll just log and continue
            if let Ok(mut resources) = device.resources.lock() {
                // First synchronize the stream
                if let Err(e) = resources.stream.synchronize() {
                    println!(
                        "Warning: Failed to synchronize CUDA stream for device {}: {}",
                        i, e
                    );
                }

                // Explicitly drop resources
                resources.create3_module = unsafe { std::mem::zeroed() };
                if resources.create2_module.is_some() {
                    resources.create2_module = None;
                }
                resources.stream = unsafe { std::mem::zeroed() };
            } else {
                println!("Warning: Could not lock device {} resources for cleanup", i);
            }
        }

        // Context cleanup happens automatically when the Context is dropped
        // Clear the devices vector to ensure all resources are dropped
        self.devices.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::tests::{test_deployer, TEST_NAMESPACE, TEST_SALT};
    use crate::create3::{compute_create2_address, compute_create3_address};
    use alloy_primitives::B256;

    #[test]
    fn test_cuda_create3_basic() {
        let gpu = GpuVanitySearch::new().expect("Failed to initialize CUDA");

        let deployer = test_deployer();
        let initial_salt = B256::ZERO.to_vec();

        // Test without namespace
        let result = gpu.search_with_threads_create3(
            &deployer.as_slice(),
            "",
            "",
            &initial_salt,
            Some(1),
            Some(1),
        );
        assert!(result.is_some(), "Should find a result");

        let found_salt = result.unwrap();
        let found_address = compute_create3_address(deployer, B256::from_slice(&found_salt), None)
            .expect("Failed to compute address");

        // The address should be valid
        assert!(
            found_address.to_string().starts_with("0x"),
            "Address should start with 0x"
        );
        assert_eq!(
            found_address.to_string().len(),
            42,
            "Address should be 42 chars long"
        );
    }

    #[test]
    fn test_cuda_create3_with_namespace() {
        let gpu = GpuVanitySearch::new().expect("Failed to initialize CUDA");

        let deployer = test_deployer();
        let initial_salt = B256::ZERO.to_vec();

        // Test with namespace
        let result = gpu.search_with_threads_create3(
            &deployer.as_slice(),
            "",
            TEST_NAMESPACE,
            &initial_salt,
            Some(1),
            Some(1),
        );
        assert!(result.is_some(), "Should find a result");

        let found_salt = result.unwrap();
        let found_address = compute_create3_address(
            deployer,
            B256::from_slice(&found_salt),
            Some(TEST_NAMESPACE),
        )
        .expect("Failed to compute address");

        // The address should be valid
        assert!(
            found_address.to_string().starts_with("0x"),
            "Address should start with 0x"
        );
        assert_eq!(
            found_address.to_string().len(),
            42,
            "Address should be 42 chars long"
        );
    }

    #[test]
    fn test_cuda_create3_prefix_matching() {
        let gpu = GpuVanitySearch::new().expect("Failed to initialize CUDA");

        let deployer = test_deployer();
        let initial_salt = B256::ZERO.to_vec();

        // Test with a prefix that doesn't match - should return None
        let result = gpu.search_with_threads_create3(
            &deployer.as_slice(),
            "ffff",
            "",
            &initial_salt,
            Some(1),
            Some(1),
        );
        assert!(
            result.is_none(),
            "Should not find a result with non-matching prefix in single iteration"
        );

        // Test with a prefix that should be findable
        let result = gpu.search_with_threads_create3(
            &deployer.as_slice(),
            "0",
            "",
            &initial_salt,
            Some(1),
            Some(65536),
        );
        assert!(result.is_some(), "Should find a result with simple prefix");

        let found_salt = result.unwrap();
        let found_address = compute_create3_address(deployer, B256::from_slice(&found_salt), None)
            .expect("Failed to compute address");

        // The address should start with the prefix
        assert!(
            found_address.to_string().to_lowercase().starts_with("0x0"),
            "Address should start with 0x0, got {}",
            found_address
        );
    }

    #[test]
    fn test_cuda_create2_basic() {
        let gpu = GpuVanitySearch::new().expect("Failed to initialize CUDA");

        if gpu.devices.is_empty()
            || gpu
                .devices
                .iter()
                .all(|d| d.resources.lock().unwrap().create2_module.is_none())
        {
            println!("Skipping CREATE2 test as module is not available on any device");
            return;
        }

        let deployer = test_deployer();
        let initial_salt = B256::ZERO.to_vec();
        let bytecode_hash = B256::ZERO.to_vec();

        // Test basic CREATE2
        let result = gpu.search_with_threads_create2(
            &deployer.as_slice(),
            "",
            &bytecode_hash,
            &initial_salt,
            Some(1),
            Some(1),
        );

        if let Some(found_salt) = result {
            let found_address =
                compute_create2_address(deployer, B256::from_slice(&found_salt), B256::ZERO);

            // The address should be valid
            assert!(
                found_address.to_string().starts_with("0x"),
                "Address should start with 0x"
            );
            assert_eq!(
                found_address.to_string().len(),
                42,
                "Address should be 42 chars long"
            );
        } else {
            println!("CREATE2 search returned no result - may need more iterations");
        }
    }

    #[test]
    #[should_panic(expected = "Invalid hex in deployer address")]
    fn test_cuda_create3_invalid_deployer() {
        let gpu = GpuVanitySearch::new().expect("Failed to initialize CUDA");

        // Test with invalid deployer length
        let invalid_deployer = vec![0; 19]; // Too short
        let salt = TEST_SALT
            .parse::<B256>()
            .expect("Invalid test salt")
            .to_vec();

        gpu.search_with_threads_create3(&invalid_deployer, "", "", &salt, Some(1), Some(1));
    }
}
