#[cfg(feature = "metal")]
use alloy_primitives::{hex, B256};
#[cfg(feature = "metal")]
use metal::*;
#[cfg(feature = "metal")]
use objc::rc::autoreleasepool;

// Test 1: Basic hex conversion
const TEST_HEX_SHADER: &str = r#"
#include <metal_stdlib>
using namespace metal;

kernel void test_hex_conversion(
    device uint8_t* input [[buffer(0)]],  // Raw bytes to convert
    device uint8_t* output [[buffer(1)]],  // Output hex values
    uint tid [[thread_position_in_grid]]
) {
    if (tid >= 20) return;  // Only process first 20 bytes
    
    uint8_t byte = input[tid];
    output[tid * 2] = (byte >> 4) & 0xF;     // High nibble
    output[tid * 2 + 1] = byte & 0xF;        // Low nibble
}
"#;

// Test 2: Prefix matching
const TEST_PREFIX_SHADER: &str = r#"
#include <metal_stdlib>
using namespace metal;

kernel void test_prefix_matching(
    device uint8_t* address [[buffer(0)]],    // Raw address bytes
    device uint8_t* prefix [[buffer(1)]],     // Prefix bytes (0-15 values)
    device uint* prefix_len [[buffer(2)]],    // Length of prefix
    device bool* matches [[buffer(3)]],       // Output match result
    uint tid [[thread_position_in_grid]]
) {
    if (tid != 0) return;  // Only one thread needs to do this
    
    bool is_match = true;
    for (uint i = 0; i < *prefix_len && is_match; i++) {
        uint8_t byte = address[i / 2];
        uint8_t hex_char;
        if (i % 2 == 0) {
            hex_char = (byte >> 4) & 0xF;
        } else {
            hex_char = byte & 0xF;
        }
        if (hex_char != prefix[i]) {
            is_match = false;
        }
    }
    *matches = is_match;
}
"#;

// Test 3: Combined hex conversion and prefix matching
const TEST_COMBINED_SHADER: &str = r#"
#include <metal_stdlib>
using namespace metal;

kernel void test_combined(
    device uint8_t* address [[buffer(0)]],    // Raw address bytes
    device uint8_t* prefix [[buffer(1)]],     // Prefix bytes (0-15 values)
    device uint* prefix_len [[buffer(2)]],    // Length of prefix
    device uint8_t* hex_output [[buffer(3)]], // Output hex values
    device bool* matches [[buffer(4)]],       // Output match result
    uint tid [[thread_position_in_grid]]
) {
    if (tid != 0) return;  // Only one thread needs to do this
    
    // First convert address to hex
    for (uint i = 0; i < 20; i++) {
        uint8_t byte = address[i];
        hex_output[i * 2] = (byte >> 4) & 0xF;     // High nibble
        hex_output[i * 2 + 1] = byte & 0xF;        // Low nibble
    }
    
    // Then check prefix match
    bool is_match = true;
    for (uint i = 0; i < *prefix_len && is_match; i++) {
        if (hex_output[i] != prefix[i]) {
            is_match = false;
        }
    }
    *matches = is_match;
}
"#;

// Test 4: Multithreaded prefix matching
const TEST_MULTITHREAD_SHADER: &str = r#"
#include <metal_stdlib>
using namespace metal;

kernel void test_multithread_matching(
    device uint8_t* addresses [[buffer(0)]],   // Multiple addresses to test (20 bytes each)
    device uint8_t* prefix [[buffer(1)]],      // Prefix bytes (0-15 values)
    device uint* prefix_len [[buffer(2)]],     // Length of prefix
    device bool* matches [[buffer(3)]],        // Output match results
    uint tid [[thread_position_in_grid]]
) {
    if (tid >= 4) return;  // Test with 4 threads
    
    // Get this thread's address (20 bytes per address)
    device uint8_t* address = addresses + (tid * 20);
    
    // Convert address to hex and check prefix
    bool is_match = true;
    for (uint i = 0; i < *prefix_len && is_match; i++) {
        uint8_t byte = address[i / 2];
        uint8_t hex_char;
        if (i % 2 == 0) {
            hex_char = (byte >> 4) & 0xF;
        } else {
            hex_char = byte & 0xF;
        }
        if (hex_char != prefix[i]) {
            is_match = false;
        }
    }
    matches[tid] = is_match;
}
"#;

// Test 5: Keccak-256 hash computation
const TEST_KECCAK_SHADER: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant ulong RC[24] = {
    0x0000000000000001UL, 0x0000000000008082UL, 0x800000000000808aUL,
    0x8000000080008000UL, 0x000000000000808bUL, 0x0000000080000001UL,
    0x8000000080008081UL, 0x8000000000008009UL, 0x000000000000008aUL,
    0x0000000000000088UL, 0x0000000080008009UL, 0x000000008000000aUL,
    0x000000008000808bUL, 0x800000000000008bUL, 0x8000000000008089UL,
    0x8000000000008003UL, 0x8000000000008002UL, 0x8000000000000080UL,
    0x000000000000800aUL, 0x800000008000000aUL, 0x8000000080008081UL,
    0x8000000000008080UL, 0x0000000080000001UL, 0x8000000080008008UL
};

inline ulong ROTL64(ulong x, int y) {
    return (x << y) | (x >> (64 - y));
}

void keccak_f(thread ulong *state) {
    int piln[24] = { 10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1 };

    ulong C[5], D;
    for (int round = 0; round < 24; round++) {
        for (int i = 0; i < 5; i++) {
            C[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }

        for (int i = 0; i < 5; i++) {
            D = C[(i + 4) % 5] ^ ROTL64(C[(i + 1) % 5], 1);
            for (int j = 0; j < 25; j += 5) {
                state[j + i] ^= D;
            }
        }

        ulong tmp = state[1];
        for (int i = 0; i < 24; i++) {
            int j = piln[i];
            C[0] = state[j];
            state[j] = ROTL64(tmp, (i + 1) * (i + 2) / 2 % 64);
            tmp = C[0];
        }

        for (int j = 0; j < 25; j += 5) {
            ulong t0 = state[j + 0], t1 = state[j + 1], t2 = state[j + 2], t3 = state[j + 3], t4 = state[j + 4];
            state[j + 0] ^= ~t1 & t2;
            state[j + 1] ^= ~t2 & t3;
            state[j + 2] ^= ~t3 & t4;
            state[j + 3] ^= ~t4 & t0;
            state[j + 4] ^= ~t0 & t1;
        }

        state[0] ^= RC[round];
    }
}

void keccak256(const device uchar *localInput, thread uchar *localOutput, thread uint inputLength)  {
    const uint rsize = 136; // 1088 bits (136 bytes) for Keccak-256
    thread ulong state[25] = {0};
    uint i = 0;
    while (i < inputLength) {
        if (i + rsize <= inputLength) {
            for (uint j = 0; j < rsize / 8; j++) {
                ulong block = 0;
                for (int k = 0; k < 8; k++) {
                    block |= (ulong)(localInput[i + j * 8 + k]) << (8 * k);
                }
                state[j] ^= block;
            }
            keccak_f(state);
            i += rsize;
        } else {
            // Handle the last block with padding
            uchar padded[rsize] = {0};
            for (uint j = 0; j < inputLength - i; j++) {
                padded[j] = localInput[i + j];
            }
            padded[inputLength - i] = 0x01; // Padding start
            padded[rsize - 1] |= 0x80; // Padding end
            for (uint j = 0; j < rsize / 8; j++) {
                ulong block = 0;
                for (int k = 0; k < 8; k++) {
                    block |= (ulong)(padded[j * 8 + k]) << (8 * k);
                }
                state[j] ^= block;
            }
            keccak_f(state);
            break;
        }
    }
    if (inputLength == 0) {
        uchar padded[rsize] = {0};
        padded[0] = 0x01;
        padded[rsize - 1] |= 0x80;
        for (uint j = 0; j < rsize / 8; j++) {
            ulong block = 0;
            for (int k = 0; k < 8; k++) {
                block |= (ulong)(padded[j * 8 + k]) << (8 * k);
            }
            state[j] ^= block;
        }
        keccak_f(state);
    }
    // Write the output
    for (uint j = 0; j < 32; j++) {
        localOutput[j] = (uchar)((state[j / 8] >> (8 * (j % 8))) & 0xFF);
    }
}

kernel void test_keccak256(
    const device uchar* input [[buffer(0)]],
    device uchar* output [[buffer(1)]],
    const device uint* input_len [[buffer(2)]],
    uint tid [[thread_position_in_grid]]
) {
    if (tid != 0) return;  // Single threaded test
    
    // Call keccak256 directly on the input message.
    // Note: keccak256 reads from device memory, so no local copy is needed.
    thread uchar localOutput[32];
    keccak256(input, localOutput, *input_len);
    
    // Write the 32-byte hash into the output buffer.
    // (Assuming the output buffer is laid out with one 32-byte hash per thread.)
    for (int i = 0; i < 32; i++) {
        output[i] = localOutput[i];
    }
}
"#;

// Test 6: CREATE3 address computation
const TEST_CREATE3_SHADER: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant ulong RC[24] = {
    0x0000000000000001UL, 0x0000000000008082UL, 0x800000000000808aUL,
    0x8000000080008000UL, 0x000000000000808bUL, 0x0000000080000001UL,
    0x8000000080008081UL, 0x8000000000008009UL, 0x000000000000008aUL,
    0x0000000000000088UL, 0x0000000080008009UL, 0x000000008000000aUL,
    0x000000008000808bUL, 0x800000000000008bUL, 0x8000000000008089UL,
    0x8000000000008003UL, 0x8000000000008002UL, 0x8000000000000080UL,
    0x000000000000800aUL, 0x800000008000000aUL, 0x8000000080008081UL,
    0x8000000000008080UL, 0x0000000080000001UL, 0x8000000080008008UL
};

inline ulong ROTL64(ulong x, int y) {
    return (x << y) | (x >> (64 - y));
}

void keccak_f(thread ulong *state) {
    int piln[24] = { 10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1 };

    ulong C[5], D;
    for (int round = 0; round < 24; round++) {
        for (int i = 0; i < 5; i++) {
            C[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }

        for (int i = 0; i < 5; i++) {
            D = C[(i + 4) % 5] ^ ROTL64(C[(i + 1) % 5], 1);
            for (int j = 0; j < 25; j += 5) {
                state[j + i] ^= D;
            }
        }

        ulong tmp = state[1];
        for (int i = 0; i < 24; i++) {
            int j = piln[i];
            C[0] = state[j];
            state[j] = ROTL64(tmp, (i + 1) * (i + 2) / 2 % 64);
            tmp = C[0];
        }

        for (int j = 0; j < 25; j += 5) {
            ulong t0 = state[j + 0], t1 = state[j + 1], t2 = state[j + 2], t3 = state[j + 3], t4 = state[j + 4];
            state[j + 0] ^= ~t1 & t2;
            state[j + 1] ^= ~t2 & t3;
            state[j + 2] ^= ~t3 & t4;
            state[j + 3] ^= ~t4 & t0;
            state[j + 4] ^= ~t0 & t1;
        }

        state[0] ^= RC[round];
    }
}

void keccak256(thread uchar *localInput, thread uchar *localOutput, thread uint inputLength)  {
    const uint rsize = 136; // 1088 bits (136 bytes) for Keccak-256
    thread ulong state[25] = {0};
    uint i = 0;
    while (i < inputLength) {
        if (i + rsize <= inputLength) {
            for (uint j = 0; j < rsize / 8; j++) {
                ulong block = 0;
                for (int k = 0; k < 8; k++) {
                    block |= (ulong)(localInput[i + j * 8 + k]) << (8 * k);
                }
                state[j] ^= block;
            }
            keccak_f(state);
            i += rsize;
        } else {
            // Handle the last block with padding
            uchar padded[rsize] = {0};
            for (uint j = 0; j < inputLength - i; j++) {
                padded[j] = localInput[i + j];
            }
            padded[inputLength - i] = 0x01; // Padding start
            padded[rsize - 1] |= 0x80; // Padding end
            for (uint j = 0; j < rsize / 8; j++) {
                ulong block = 0;
                for (int k = 0; k < 8; k++) {
                    block |= (ulong)(padded[j * 8 + k]) << (8 * k);
                }
                state[j] ^= block;
            }
            keccak_f(state);
            break;
        }
    }
    if (inputLength == 0) {
        uchar padded[rsize] = {0};
        padded[0] = 0x01;
        padded[rsize - 1] |= 0x80;
        for (uint j = 0; j < rsize / 8; j++) {
            ulong block = 0;
            for (int k = 0; k < 8; k++) {
                block |= (ulong)(padded[j * 8 + k]) << (8 * k);
            }
            state[j] ^= block;
        }
        keccak_f(state);
    }
    // Write the output
    for (uint j = 0; j < 32; j++) {
        localOutput[j] = (uchar)((state[j / 8] >> (8 * (j % 8))) & 0xFF);
    }
}

constant uchar CREATE3_FACTORY[20] = {
    0x00, 0x4e, 0xe0, 0x12, 0xd7, 0x7c, 0x5d, 0x0e,
    0x67, 0xd8, 0x61, 0x04, 0x1d, 0x11, 0x82, 0x4f,
    0x51, 0xb5, 0x90, 0xfb
};

kernel void test_create3_address(
    const device uchar* deployer [[buffer(0)]],
    const device uchar* salt [[buffer(1)]],
    const device uchar* ns [[buffer(2)]],
    const device uint* ns_len [[buffer(3)]],
    device uchar* final_address [[buffer(4)]],
    uint tid [[thread_position_in_grid]]
) {
    if (tid != 0) return;  // Single threaded test

    thread uchar FACTORY_BYTECODE[16] = {
        0x67, 0x36, 0x3d, 0x3d, 0x37, 0x36, 0x3d, 0x34,
        0xf0, 0x3d, 0x52, 0x60, 0x08, 0x60, 0x18, 0xf3
    };
    
    // First compute namespaced salt (no namespace for test)
    thread uchar input[256];
    uint input_len = 0;

    thread uchar namespaced_salt[32];

    if (*ns_len > 0) {
        // Add salt
        for (uint i = 0; i < 32; i++) {
            input[input_len++] = salt[i];
        }
        // Add namespace
        for (uint i = 0; i < *ns_len; i++) {
            input[input_len++] = ns[i];
        }
        
        keccak256(input, namespaced_salt, input_len);
    } else {
        for (uint i = 0; i < 32; i++) {
            namespaced_salt[i] = salt[i];
        }
    }
    
    // Then hash with deployer
    input_len = 0;
    for (uint i = 0; i < 20; i++) {
        input[input_len++] = deployer[i];
    }
    for (uint i = 0; i < 32; i++) {
        input[input_len++] = namespaced_salt[i];
    }
    
    thread uchar final_salt[32];
    keccak256(input, final_salt, input_len);
    
    // Compute CREATE2 address for proxy
    input_len = 0;
    input[input_len++] = 0xff;  // CREATE2 prefix
    for (uint i = 0; i < 20; i++) {
        input[input_len++] = CREATE3_FACTORY[i];
    }
    for (uint i = 0; i < 32; i++) {
        input[input_len++] = final_salt[i];
    }
    
    thread uchar bytecode_hash[32];
    keccak256(FACTORY_BYTECODE, bytecode_hash, 16);
    
    for (uint i = 0; i < 32; i++) {
        input[input_len++] = bytecode_hash[i];
    }
    
    thread uchar proxy_address[32];
    keccak256(input, proxy_address, input_len);
    
    // Finally compute CREATE address for contract
    input_len = 0;
    input[input_len++] = 0xd6;  // RLP prefix
    input[input_len++] = 0x94;  // Address marker
    for (uint i = 0; i < 20; i++) {
        input[input_len++] = proxy_address[12 + i];
    }
    input[input_len++] = 0x01;  // Nonce
    
    thread uchar contract_address[32];
    keccak256(input, contract_address, input_len);
    
    // Copy final address (last 20 bytes)
    for (uint i = 0; i < 20; i++) {
        final_address[i] = contract_address[12 + i];
    }
}
"#;

pub struct ShaderTester {
    device: Device,
    command_queue: CommandQueue,
}

impl ShaderTester {
    pub fn new() -> Option<Self> {
        autoreleasepool(|| {
            let device = Device::system_default()?;
            println!("Found Metal device: {}", device.name());
            let command_queue = device.new_command_queue();
            Some(Self {
                device,
                command_queue,
            })
        })
    }

    pub fn test_hex_conversion(&self, input_bytes: &[u8]) -> Vec<u8> {
        autoreleasepool(|| {
            // Create input buffer
            let input_buffer = self.device.new_buffer_with_data(
                input_bytes.as_ptr() as *const _,
                input_bytes.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            // Create output buffer (2 hex chars per byte)
            let output_buffer = self.device.new_buffer(
                (input_bytes.len() * 2) as u64,
                MTLResourceOptions::StorageModeShared,
            );

            // Create pipeline
            let library = self
                .device
                .new_library_with_source(TEST_HEX_SHADER, &CompileOptions::new())
                .expect("Failed to create hex test library");
            let function = library
                .get_function("test_hex_conversion", None)
                .expect("Failed to get hex test function");
            let pipeline = self
                .device
                .new_compute_pipeline_state_with_function(&function)
                .expect("Failed to create hex test pipeline");

            // Create command buffer and encoder
            let command_buffer = self.command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();

            // Set up the pipeline
            encoder.set_compute_pipeline_state(&pipeline);
            encoder.set_buffer(0, Some(&input_buffer), 0);
            encoder.set_buffer(1, Some(&output_buffer), 0);

            // Configure and dispatch
            let threads = metal::MTLSize::new(input_bytes.len() as u64, 1, 1);
            let threadgroups = metal::MTLSize::new(1, 1, 1);
            encoder.dispatch_thread_groups(threadgroups, threads);

            // Execute
            encoder.end_encoding();
            command_buffer.commit();
            command_buffer.wait_until_completed();

            // Get results
            unsafe {
                let output_ptr = output_buffer.contents() as *const u8;
                let output_slice = std::slice::from_raw_parts(output_ptr, input_bytes.len() * 2);
                output_slice.to_vec()
            }
        })
    }

    pub fn test_prefix_matching(&self, address: &[u8], prefix: &[u8]) -> bool {
        autoreleasepool(|| {
            // Create buffers
            let address_buffer = self.device.new_buffer_with_data(
                address.as_ptr() as *const _,
                address.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let prefix_buffer = self.device.new_buffer_with_data(
                prefix.as_ptr() as *const _,
                prefix.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let prefix_len = prefix.len() as u32;
            let prefix_len_buffer = self.device.new_buffer_with_data(
                &prefix_len as *const u32 as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let matches_buffer = self.device.new_buffer(
                std::mem::size_of::<bool>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            // Create pipeline
            let library = self
                .device
                .new_library_with_source(TEST_PREFIX_SHADER, &CompileOptions::new())
                .expect("Failed to create prefix test library");
            let function = library
                .get_function("test_prefix_matching", None)
                .expect("Failed to get prefix test function");
            let pipeline = self
                .device
                .new_compute_pipeline_state_with_function(&function)
                .expect("Failed to create prefix test pipeline");

            // Create command buffer and encoder
            let command_buffer = self.command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();

            // Set up the pipeline
            encoder.set_compute_pipeline_state(&pipeline);
            encoder.set_buffer(0, Some(&address_buffer), 0);
            encoder.set_buffer(1, Some(&prefix_buffer), 0);
            encoder.set_buffer(2, Some(&prefix_len_buffer), 0);
            encoder.set_buffer(3, Some(&matches_buffer), 0);

            // Configure and dispatch
            let threads = metal::MTLSize::new(1, 1, 1);
            let threadgroups = metal::MTLSize::new(1, 1, 1);
            encoder.dispatch_thread_groups(threadgroups, threads);

            // Execute
            encoder.end_encoding();
            command_buffer.commit();
            command_buffer.wait_until_completed();

            // Get result
            unsafe { *(matches_buffer.contents() as *const bool) }
        })
    }

    pub fn test_combined(&self, address: &[u8], prefix: &[u8]) -> (Vec<u8>, bool) {
        autoreleasepool(|| {
            // Create buffers
            let address_buffer = self.device.new_buffer_with_data(
                address.as_ptr() as *const _,
                address.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let prefix_buffer = self.device.new_buffer_with_data(
                prefix.as_ptr() as *const _,
                prefix.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let prefix_len = prefix.len() as u32;
            let prefix_len_buffer = self.device.new_buffer_with_data(
                &prefix_len as *const u32 as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let hex_output_buffer = self.device.new_buffer(
                40, // 20 bytes * 2 hex chars each
                MTLResourceOptions::StorageModeShared,
            );

            let matches_buffer = self.device.new_buffer(
                std::mem::size_of::<bool>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            // Create pipeline
            let library = self
                .device
                .new_library_with_source(TEST_COMBINED_SHADER, &CompileOptions::new())
                .expect("Failed to create combined test library");
            let function = library
                .get_function("test_combined", None)
                .expect("Failed to get combined test function");
            let pipeline = self
                .device
                .new_compute_pipeline_state_with_function(&function)
                .expect("Failed to create combined test pipeline");

            // Create command buffer and encoder
            let command_buffer = self.command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();

            // Set up the pipeline
            encoder.set_compute_pipeline_state(&pipeline);
            encoder.set_buffer(0, Some(&address_buffer), 0);
            encoder.set_buffer(1, Some(&prefix_buffer), 0);
            encoder.set_buffer(2, Some(&prefix_len_buffer), 0);
            encoder.set_buffer(3, Some(&hex_output_buffer), 0);
            encoder.set_buffer(4, Some(&matches_buffer), 0);

            // Configure and dispatch
            let threads = metal::MTLSize::new(1, 1, 1);
            let threadgroups = metal::MTLSize::new(1, 1, 1);
            encoder.dispatch_thread_groups(threadgroups, threads);

            // Execute
            encoder.end_encoding();
            command_buffer.commit();
            command_buffer.wait_until_completed();

            // Get results
            unsafe {
                let hex_output =
                    std::slice::from_raw_parts(hex_output_buffer.contents() as *const u8, 40)
                        .to_vec();
                let matches = *(matches_buffer.contents() as *const bool);
                (hex_output, matches)
            }
        })
    }

    pub fn test_multithread_matching(&self, addresses: &[u8], prefix: &[u8]) -> Vec<bool> {
        autoreleasepool(|| {
            // Create buffers
            let addresses_buffer = self.device.new_buffer_with_data(
                addresses.as_ptr() as *const _,
                addresses.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let prefix_buffer = self.device.new_buffer_with_data(
                prefix.as_ptr() as *const _,
                prefix.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let prefix_len = prefix.len() as u32;
            let prefix_len_buffer = self.device.new_buffer_with_data(
                &prefix_len as *const u32 as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let matches_buffer = self.device.new_buffer(
                4 * std::mem::size_of::<bool>() as u64, // 4 threads
                MTLResourceOptions::StorageModeShared,
            );

            // Create pipeline
            let library = self
                .device
                .new_library_with_source(TEST_MULTITHREAD_SHADER, &CompileOptions::new())
                .expect("Failed to create multithread test library");
            let function = library
                .get_function("test_multithread_matching", None)
                .expect("Failed to get multithread test function");
            let pipeline = self
                .device
                .new_compute_pipeline_state_with_function(&function)
                .expect("Failed to create multithread test pipeline");

            // Create command buffer and encoder
            let command_buffer = self.command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();

            // Set up the pipeline
            encoder.set_compute_pipeline_state(&pipeline);
            encoder.set_buffer(0, Some(&addresses_buffer), 0);
            encoder.set_buffer(1, Some(&prefix_buffer), 0);
            encoder.set_buffer(2, Some(&prefix_len_buffer), 0);
            encoder.set_buffer(3, Some(&matches_buffer), 0);

            // Configure and dispatch
            let threads = metal::MTLSize::new(4, 1, 1); // 4 threads
            let threadgroups = metal::MTLSize::new(1, 1, 1);
            encoder.dispatch_thread_groups(threadgroups, threads);

            // Execute
            encoder.end_encoding();
            command_buffer.commit();
            command_buffer.wait_until_completed();

            // Get results
            unsafe {
                let matches_ptr = matches_buffer.contents() as *const bool;
                let matches_slice = std::slice::from_raw_parts(matches_ptr, 4);
                matches_slice.to_vec()
            }
        })
    }

    pub fn test_keccak256(&self, input: &[u8]) -> Vec<u8> {
        autoreleasepool(|| {
            println!("test_keccak256: input buffer");
            // Create input buffer with at least 1 byte
            let input_buffer = if input.is_empty() {
                self.device
                    .new_buffer(1, MTLResourceOptions::StorageModeShared)
            } else {
                self.device.new_buffer_with_data(
                    input.as_ptr() as *const _,
                    input.len() as u64,
                    MTLResourceOptions::StorageModeShared,
                )
            };

            println!("test_keccak256: output buffer");
            let output_buffer = self.device.new_buffer(
                32, // 256 bits = 32 bytes
                MTLResourceOptions::StorageModeShared,
            );

            println!("test_keccak256: input length");
            let input_len = input.len() as u32;
            let input_len_buffer = self.device.new_buffer_with_data(
                &input_len as *const u32 as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            println!("test_keccak256: library");
            let library = self
                .device
                .new_library_with_source(TEST_KECCAK_SHADER, &CompileOptions::new())
                .expect("Failed to create Keccak test library");
            println!("test_keccak256: library created");
            let function = library
                .get_function("test_keccak256", None)
                .expect("Failed to get Keccak test function");
            println!("test_keccak256: function created");
            let pipeline = self
                .device
                .new_compute_pipeline_state_with_function(&function)
                .expect("Failed to create Keccak test pipeline");
            println!("test_keccak256: pipeline created");

            let command_buffer = self.command_queue.new_command_buffer();
            println!("test_keccak256: command buffer");
            let mut encoder = command_buffer.new_compute_command_encoder();
            println!("test_keccak256: encoder");

            encoder.set_compute_pipeline_state(&pipeline);
            println!("test_keccak256: pipeline set");
            encoder.set_buffer(0, Some(&input_buffer), 0);
            println!("test_keccak256: input length buffer set");
            encoder.set_buffer(1, Some(&output_buffer), 0);
            println!("test_keccak256: input buffer set");
            encoder.set_buffer(2, Some(&input_len_buffer), 0);

            let threads = metal::MTLSize::new(1, 1, 1);
            let threadgroups = metal::MTLSize::new(1, 1, 1);
            encoder.dispatch_thread_groups(threadgroups, threads);

            encoder.end_encoding();
            command_buffer.commit();
            command_buffer.wait_until_completed();

            unsafe {
                let output_ptr = output_buffer.contents() as *const u8;
                let output_slice = std::slice::from_raw_parts(output_ptr, 32);
                output_slice.to_vec()
            }
        })
    }

    pub fn test_create3_address(&self, deployer: &[u8], salt: &[u8], namespace: &[u8]) -> Vec<u8> {
        println!("test create3 address -1");
        autoreleasepool(|| {
            println!("test create3 address 0");
            let deployer_buffer = self.device.new_buffer_with_data(
                deployer.as_ptr() as *const _,
                deployer.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let salt_buffer = self.device.new_buffer_with_data(
                salt.as_ptr() as *const _,
                salt.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let namespace_buffer = if namespace.is_empty() {
                self.device
                    .new_buffer(1, MTLResourceOptions::StorageModeShared)
            } else {
                self.device.new_buffer_with_data(
                    namespace.as_ptr() as *const _,
                    namespace.len() as u64,
                    MTLResourceOptions::StorageModeShared,
                )
            };

            let namespace_len = namespace.len() as u32;
            let namespace_len_buffer = self.device.new_buffer_with_data(
                &namespace_len as *const u32 as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let address_buffer = self.device.new_buffer(
                20, // Address is 20 bytes
                MTLResourceOptions::StorageModeShared,
            );

            println!("test_create3_address: buffers created");

            let library = self
                .device
                .new_library_with_source(TEST_CREATE3_SHADER, &CompileOptions::new())
                .expect("Failed to create CREATE3 test library");
            println!("test_create3_address: library created");
            let function = library
                .get_function("test_create3_address", None)
                .expect("Failed to get CREATE3 test function");
            println!("test_create3_address: function created");
            let pipeline = self
                .device
                .new_compute_pipeline_state_with_function(&function)
                .expect("Failed to create CREATE3 test pipeline");
            println!("test_create3_address: pipeline created");

            let command_buffer = self.command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();

            println!("test create3 address 3");

            encoder.set_compute_pipeline_state(&pipeline);

            println!("test create3 address 4");
            encoder.set_buffer(0, Some(&deployer_buffer), 0);
            encoder.set_buffer(1, Some(&salt_buffer), 0);
            encoder.set_buffer(2, Some(&namespace_buffer), 0);
            encoder.set_buffer(3, Some(&namespace_len_buffer), 0);
            encoder.set_buffer(4, Some(&address_buffer), 0);

            println!("test create3 address 5");

            let threads = metal::MTLSize::new(1, 1, 1);
            let threadgroups = metal::MTLSize::new(1, 1, 1);
            encoder.dispatch_thread_groups(threadgroups, threads);

            encoder.end_encoding();
            command_buffer.commit();
            command_buffer.wait_until_completed();

            unsafe {
                let address_ptr = address_buffer.contents() as *const u8;
                let address_slice = std::slice::from_raw_parts(address_ptr, 20);
                address_slice.to_vec()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::tests::{test_deployer, TEST_SALT};

    #[test]
    fn test_hex_conversion() {
        let tester = ShaderTester::new().expect("Failed to create shader tester");

        // Test case 1: Simple byte
        let input = vec![0xAB];
        let output = tester.test_hex_conversion(&input);
        assert_eq!(output, vec![0x0A, 0x0B], "Failed to convert 0xAB");

        // Test case 2: Multiple bytes
        let input = vec![0x12, 0x34, 0x56];
        let output = tester.test_hex_conversion(&input);
        assert_eq!(
            output,
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
            "Failed to convert multiple bytes"
        );

        // Test case 3: Zero byte
        let input = vec![0x00];
        let output = tester.test_hex_conversion(&input);
        assert_eq!(output, vec![0x00, 0x00], "Failed to convert 0x00");
    }

    #[test]
    fn test_prefix_matching() {
        let tester = ShaderTester::new().expect("Failed to create shader tester");

        // Test case 1: Exact match
        let address = vec![0x00, 0x00]; // Address starting with "0000"
        let prefix = vec![0x0, 0x0, 0x0, 0x0]; // Looking for "0000"
        assert!(
            tester.test_prefix_matching(&address, &prefix),
            "Failed to match exact prefix"
        );

        // Test case 2: No match
        let address = vec![0x12, 0x34]; // Address starting with "1234"
        let prefix = vec![0x0, 0x0, 0x0, 0x0]; // Looking for "0000"
        assert!(
            !tester.test_prefix_matching(&address, &prefix),
            "Incorrectly matched non-matching prefix"
        );

        // Test case 3: Partial match
        let address = vec![0x00, 0x12]; // Address starting with "0012"
        let prefix = vec![0x0, 0x0]; // Looking for "00"
        assert!(
            tester.test_prefix_matching(&address, &prefix),
            "Failed to match partial prefix"
        );
    }

    #[test]
    fn test_combined_conversion_and_matching() {
        let tester = ShaderTester::new().expect("Failed to create shader tester");

        // Test case 1: Address that should match
        let address = vec![0x00, 0x00]; // Address starting with "0000"
        let prefix = vec![0x0, 0x0, 0x0, 0x0]; // Looking for "0000"
        let (hex_output, matches) = tester.test_combined(&address, &prefix);
        println!("Test case 1:");
        println!("Address bytes: {:02x?}", address);
        println!("Prefix bytes: {:02x?}", prefix);
        println!("Hex output: {:02x?}", &hex_output[..8]);
        println!("Matches: {}", matches);
        assert!(matches, "Failed to match valid prefix");
        assert_eq!(
            &hex_output[..4],
            &[0x0, 0x0, 0x0, 0x0],
            "Incorrect hex conversion"
        );

        // Test case 2: Address that shouldn't match
        let address = vec![0x12, 0x34]; // Address starting with "1234"
        let prefix = vec![0x0, 0x0, 0x0, 0x0]; // Looking for "0000"
        let (hex_output, matches) = tester.test_combined(&address, &prefix);
        println!("\nTest case 2:");
        println!("Address bytes: {:02x?}", address);
        println!("Prefix bytes: {:02x?}", prefix);
        println!("Hex output: {:02x?}", &hex_output[..8]);
        println!("Matches: {}", matches);
        assert!(!matches, "Incorrectly matched invalid prefix");
        assert_eq!(
            &hex_output[..4],
            &[0x1, 0x2, 0x3, 0x4],
            "Incorrect hex conversion"
        );
    }

    #[test]
    fn test_multithread_matching() {
        let tester = ShaderTester::new().expect("Failed to create shader tester");

        // Create test addresses (80 bytes total = 4 addresses * 20 bytes each)
        let mut addresses = Vec::with_capacity(80);

        // Address 1: Starts with 0000
        addresses.extend_from_slice(&[0x00, 0x00, 0x12, 0x34]);
        addresses.extend_from_slice(&[0; 16]); // Pad to 20 bytes

        // Address 2: Starts with 1234
        addresses.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        addresses.extend_from_slice(&[0; 16]); // Pad to 20 bytes

        // Address 3: Starts with 0000
        addresses.extend_from_slice(&[0x00, 0x00, 0x89, 0xAB]);
        addresses.extend_from_slice(&[0; 16]); // Pad to 20 bytes

        // Address 4: Starts with FFFF
        addresses.extend_from_slice(&[0xFF, 0xFF, 0xCD, 0xEF]);
        addresses.extend_from_slice(&[0; 16]); // Pad to 20 bytes

        // Test with prefix "0000"
        let prefix = vec![0x0, 0x0, 0x0, 0x0];
        let results = tester.test_multithread_matching(&addresses, &prefix);

        println!("Multithread test results:");
        println!("Address 1 (0000...): {}", results[0]);
        println!("Address 2 (1234...): {}", results[1]);
        println!("Address 3 (0000...): {}", results[2]);
        println!("Address 4 (FFFF...): {}", results[3]);

        assert_eq!(results.len(), 4, "Should have 4 results");
        assert!(results[0], "First address should match");
        assert!(!results[1], "Second address should not match");
        assert!(results[2], "Third address should match");
        assert!(!results[3], "Fourth address should not match");
    }

    #[test]
    fn test_keccak256_hash() {
        let tester = ShaderTester::new().expect("Failed to create shader tester");

        // Test case 1: Empty input
        let input = vec![];
        let output = tester.test_keccak256(&input);
        let expected =
            hex::decode("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
                .unwrap();
        assert_eq!(output, expected, "Failed to hash empty input");

        // Test case 2: Simple input
        let input = b"hello".to_vec();
        let output = tester.test_keccak256(&input);
        let expected =
            hex::decode("1c8aff950685c2ed4bc3174f3472287b56d9517b9c948127319a09a7a36deac8")
                .unwrap();
        assert_eq!(output, expected, "Failed to hash 'hello'");

        // Test case 3: Long input
        let input = b"The quick brown fox jumps over the lazy dog".to_vec();
        let output = tester.test_keccak256(&input);
        let expected =
            hex::decode("4d741b6f1eb29cb2a9b9911c82f56fa8d73b04959d3d9d222895df6c0b28aa15")
                .unwrap();
        assert_eq!(output, expected, "Failed to hash long input");
    }

    #[test]
    fn test_create3_address_computation() {
        println!("test_create3_address_computation");
        let tester = ShaderTester::new().expect("Failed to create shader tester");

        // Test with known values from TypeScript implementation
        println!("test_create3_address_computation 1");
        let deployer = test_deployer().to_vec();
        println!("test_create3_address_computation 2");
        let salt = TEST_SALT
            .parse::<B256>()
            .expect("Invalid test salt")
            .to_vec();
        println!("test_create3_address_computation 3");

        let address = tester.test_create3_address(&deployer, &salt, &[]);
        println!("test_create3_address_computation 4");
        let address_hex = format!("0x{}", hex::encode(&address));
        println!("test_create3_address_computation 5");

        assert_eq!(
            address_hex.to_lowercase(),
            "0xAB6528783ac2a0BEf235ada3E1A5F6d8a623867E".to_lowercase(),
            "CREATE3 address does not match TypeScript implementation"
        );

        let namespace = b"L2ReverseRegistrar v1.0.0";
        let address = tester.test_create3_address(
            &deployer,
            &hex::decode("0x7cc6b9a2afa05a889a0394c767107d001d86bf77bea0141c11c296d3a8f72dac")
                .unwrap(),
            namespace,
        );
        let address_hex = format!("0x{}", hex::encode(&address));

        assert_eq!(
            address_hex.to_lowercase(),
            "0x5678e8193257f8b43eea3c5873b59ebcc18a0043".to_lowercase(),
            "CREATE3 address does not match TypeScript implementation"
        );
    }
}

#[cfg(feature = "metal")]
pub fn test_shader() {
    autoreleasepool(|| {
        let device = Device::system_default().unwrap();
        println!("Found Metal device: {}", device.name());

        let shader_src = include_str!("shader/CREATE3.metal");
        let library = device
            .new_library_with_source(shader_src, &metal::CompileOptions::new())
            .unwrap();

        let function = library.get_function("vanity_search", None).unwrap();

        let pipeline_state = device
            .new_compute_pipeline_state_with_function(&function)
            .unwrap();

        let command_queue = device.new_command_queue();
        println!("Command queue created");

        let deployer = hex::decode("0x4e0e12d77c5d0e67d861041d11824f51b590fb").unwrap();
        let prefix = "0000";
        let namespace = "test";
        let initial_salt = B256::ZERO.as_slice();

        // Create buffers
        let deployer_buffer = device.new_buffer_with_data(
            deployer.as_ptr() as *const _,
            deployer.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let prefix_bytes: Vec<u8> = prefix
            .chars()
            .map(|c| c.to_digit(16).unwrap() as u8)
            .collect();
        let prefix_buffer = device.new_buffer_with_data(
            prefix_bytes.as_ptr() as *const _,
            prefix_bytes.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let prefix_len = prefix_bytes.len() as u32;
        let prefix_len_buffer = device.new_buffer_with_data(
            &prefix_len as *const u32 as *const _,
            std::mem::size_of::<u32>() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let namespace_buffer = device.new_buffer_with_data(
            namespace.as_ptr() as *const _,
            namespace.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let namespace_len = namespace.len() as u32;
        let namespace_len_buffer = device.new_buffer_with_data(
            &namespace_len as *const u32 as *const _,
            std::mem::size_of::<u32>() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let initial_salt_buffer = device.new_buffer_with_data(
            initial_salt.as_ptr() as *const _,
            initial_salt.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let result_buffer = device.new_buffer(
            std::mem::size_of::<u64>() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let found_buffer = device.new_buffer(
            std::mem::size_of::<bool>() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        // Initialize found buffer to false
        unsafe {
            *(found_buffer.contents() as *mut bool) = false;
        }

        // Create command buffer
        let command_buffer = command_queue.new_command_buffer();

        // Create compute encoder
        let compute_encoder = command_buffer.new_compute_command_encoder();

        // Set pipeline state and buffers
        compute_encoder.set_compute_pipeline_state(&pipeline_state);
        compute_encoder.set_buffer(0, Some(&deployer_buffer), 0);
        compute_encoder.set_buffer(1, Some(&prefix_buffer), 0);
        compute_encoder.set_buffer(2, Some(&prefix_len_buffer), 0);
        compute_encoder.set_buffer(3, Some(&namespace_buffer), 0);
        compute_encoder.set_buffer(4, Some(&namespace_len_buffer), 0);
        compute_encoder.set_buffer(5, Some(&initial_salt_buffer), 0);
        compute_encoder.set_buffer(6, Some(&result_buffer), 0);
        compute_encoder.set_buffer(7, Some(&found_buffer), 0);

        // Configure thread groups
        let threads_per_threadgroup = metal::MTLSize::new(64, 1, 1);
        let threadgroups = metal::MTLSize::new(65536, 1, 1); // Much larger search space

        // Dispatch work
        compute_encoder.dispatch_thread_groups(threadgroups, threads_per_threadgroup);

        // End encoding and commit
        compute_encoder.end_encoding();
        command_buffer.commit();

        // Wait for completion
        command_buffer.wait_until_completed();

        // Get result
        unsafe {
            let found = *(found_buffer.contents() as *const bool);
            let salt = std::slice::from_raw_parts(result_buffer.contents() as *const u8, 32);
            println!("Found: {}", found);
            println!("Salt: {:02x?}", salt);
        }
    });
}
