#[cfg(feature = "cuda")]
mod cuda;
#[cfg(feature = "metal")]
mod metal_search;

#[derive(Debug)]
pub enum GpuBackend {
    #[cfg(feature = "metal")]
    Metal,
    #[cfg(feature = "cuda")]
    Cuda,
}

pub struct GpuVanitySearch {
    backend: GpuBackend,
    #[cfg(feature = "metal")]
    metal_search: Option<metal_search::GpuVanitySearch>,
    #[cfg(feature = "cuda")]
    cuda_search: Option<cuda::GpuVanitySearch>,
}

impl GpuVanitySearch {
    pub fn new() -> Option<Self> {
        // Try CUDA first, then fall back to Metal
        #[cfg(feature = "cuda")]
        if let Some(cuda_search) = cuda::GpuVanitySearch::new() {
            println!("Using CUDA GPU backend");
            return Some(Self {
                backend: GpuBackend::Cuda,
                #[cfg(feature = "metal")]
                metal_search: None,
                cuda_search: Some(cuda_search),
            });
        }
        #[cfg(feature = "metal")]
        {
            if let Some(metal_search) = metal_search::GpuVanitySearch::new() {
                println!("Using Metal GPU backend");
                return Some(Self {
                    backend: GpuBackend::Metal,
                    metal_search: Some(metal_search),
                    #[cfg(feature = "cuda")]
                    cuda_search: None,
                });
            }
        }
        println!("No GPU backend available");
        None
    }

    pub fn new_with_backend(backend: GpuBackend) -> Option<Self> {
        match backend {
            #[cfg(feature = "cuda")]
            GpuBackend::Cuda => {
                if let Some(cuda_search) = cuda::GpuVanitySearch::new() {
                    println!("Using CUDA GPU backend");
                    Some(Self {
                        backend: GpuBackend::Cuda,
                        #[cfg(feature = "metal")]
                        metal_search: None,
                        cuda_search: Some(cuda_search),
                    })
                } else {
                    println!("CUDA GPU backend not available");
                    None
                }
            }
            #[cfg(feature = "metal")]
            GpuBackend::Metal => {
                if let Some(metal_search) = metal_search::GpuVanitySearch::new() {
                    println!("Using Metal GPU backend");
                    Some(Self {
                        backend: GpuBackend::Metal,
                        metal_search: Some(metal_search),
                        #[cfg(feature = "cuda")]
                        cuda_search: None,
                    })
                } else {
                    println!("Metal GPU backend not available");
                    None
                }
            }
        }
    }

    // Legacy wrapper for CREATE3 compatibility
    pub fn search(
        &self,
        deployer: &[u8],
        prefix: &str,
        namespace: &str,
        initial_salt: &[u8],
        threads: Option<u32>,
        thread_groups: Option<u32>,
    ) -> Option<Vec<u8>> {
        self.search_create3(
            deployer,
            prefix,
            namespace,
            initial_salt,
            threads,
            thread_groups,
        )
    }

    pub fn search_create3(
        &self,
        deployer: &[u8],
        prefix: &str,
        namespace: &str,
        initial_salt: &[u8],
        threads: Option<u32>,
        thread_groups: Option<u32>,
    ) -> Option<Vec<u8>> {
        match self.backend {
            #[cfg(feature = "cuda")]
            GpuBackend::Cuda => {
                let threads = threads.unwrap_or(256);
                let thread_groups = thread_groups.unwrap_or(65536);
                self.cuda_search.as_ref().and_then(|cs| {
                    cs.search_with_threads_create3(
                        deployer,
                        prefix,
                        namespace,
                        initial_salt,
                        threads,
                        thread_groups,
                    )
                })
            }
            #[cfg(feature = "metal")]
            GpuBackend::Metal => {
                let threads = threads.unwrap_or(64);
                let thread_groups = thread_groups.unwrap_or(65536);
                self.metal_search
                    .as_ref()
                    .and_then(|ms| {
                        ms.search_with_threads_create3(
                            deployer,
                            prefix,
                            namespace,
                            initial_salt,
                            threads,
                            thread_groups,
                        )
                    })
                    .map(|salt| salt.to_vec())
            }
        }
    }

    pub fn search_create2(
        &self,
        deployer: &[u8],
        prefix: &str,
        bytecode_hash: &[u8],
        initial_salt: &[u8],
        threads: Option<u32>,
        thread_groups: Option<u32>,
    ) -> Option<Vec<u8>> {
        match self.backend {
            #[cfg(feature = "cuda")]
            GpuBackend::Cuda => {
                let threads = threads.unwrap_or(256);
                let thread_groups = thread_groups.unwrap_or(65536);
                self.cuda_search.as_ref().and_then(|cs| {
                    cs.search_with_threads_create2(
                        deployer,
                        prefix,
                        bytecode_hash,
                        initial_salt,
                        threads,
                        thread_groups,
                    )
                })
            }
            #[cfg(feature = "metal")]
            GpuBackend::Metal => {
                let threads = threads.unwrap_or(64);
                let thread_groups = thread_groups.unwrap_or(65536);
                self.metal_search
                    .as_ref()
                    .and_then(|ms| {
                        ms.search_with_threads_create2(
                            deployer,
                            prefix,
                            bytecode_hash,
                            initial_salt,
                            threads,
                            thread_groups,
                        )
                    })
                    .map(|salt| salt.to_vec())
            }
        }
    }
}
