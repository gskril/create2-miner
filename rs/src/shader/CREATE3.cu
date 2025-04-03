#include <cuda_runtime.h>

__constant__ unsigned long long RC[24] = {
    0x0000000000000001ULL, 0x0000000000008082ULL, 0x800000000000808aULL,
    0x8000000080008000ULL, 0x000000000000808bULL, 0x0000000080000001ULL,
    0x8000000080008081ULL, 0x8000000000008009ULL, 0x000000000000008aULL,
    0x0000000000000088ULL, 0x0000000080008009ULL, 0x000000008000000aULL,
    0x000000008000808bULL, 0x800000000000008bULL, 0x8000000000008089ULL,
    0x8000000000008003ULL, 0x8000000000008002ULL, 0x8000000000000080ULL,
    0x000000000000800aULL, 0x800000008000000aULL, 0x8000000080008081ULL,
    0x8000000000008080ULL, 0x0000000080000001ULL, 0x8000000080008008ULL
};

__device__ inline unsigned long long ROTL64(unsigned long long x, int y) {
    return (x << y) | (x >> (64 - y));
}

__device__ void keccak_f(unsigned long long *state) {
    const int rotc[24] = {
        1,  3,  6,  10, 15, 21, 28, 36, 45, 55, 2,  14,
        27, 41, 56, 8,  25, 43, 62, 18, 39, 61, 20, 44
    };
    const int piln[24] = {
        10, 7,  11, 17, 18, 3, 5,  16, 8,  21, 24, 4,
        15, 23, 19, 13, 12, 2, 20, 14, 22, 9,  6,  1
    };

    unsigned long long t, bc[5];
    
    for (int round = 0; round < 24; round++) {
        // Theta
        for (int i = 0; i < 5; i++) {
            bc[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }

        for (int i = 0; i < 5; i++) {
            t = bc[(i + 4) % 5] ^ ROTL64(bc[(i + 1) % 5], 1);
            for (int j = 0; j < 25; j += 5) {
                state[j + i] ^= t;
            }
        }

        // Rho Pi
        t = state[1];
        for (int i = 0; i < 24; i++) {
            int j = piln[i];
            bc[0] = state[j];
            state[j] = ROTL64(t, rotc[i]);
            t = bc[0];
        }

        // Chi
        for (int j = 0; j < 25; j += 5) {
            for (int i = 0; i < 5; i++) {
                bc[i] = state[j + i];
            }
            for (int i = 0; i < 5; i++) {
                state[j + i] ^= (~bc[(i + 1) % 5]) & bc[(i + 2) % 5];
            }
        }

        // Iota
        state[0] ^= RC[round];
    }
}

__device__ void keccak256(unsigned char *input, unsigned char *output, unsigned int inputLength) {
    const unsigned int rsize = 136; // 1088 bits (136 bytes) for Keccak-256
    unsigned long long state[25] = {0};
    unsigned int i;

    // Absorb input
    for (i = 0; i + rsize <= inputLength; i += rsize) {
        for (unsigned int j = 0; j < rsize / 8; j++) {
            state[j] ^= ((unsigned long long*)&input[i])[j];
        }
        keccak_f(state);
    }

    // Handle remaining input and padding
    unsigned char last_block[rsize] = {0};
    unsigned int remaining = inputLength - i;
    if (remaining > 0) {
        for (unsigned int j = 0; j < remaining; j++) {
            last_block[j] = input[i + j];
        }
    }
    last_block[remaining] = 0x01;
    last_block[rsize - 1] |= 0x80;

    for (unsigned int j = 0; j < rsize / 8; j++) {
        state[j] ^= ((unsigned long long*)last_block)[j];
    }

    keccak_f(state);

    // Extract output
    for (unsigned int j = 0; j < 32; j++) {
        output[j] = ((unsigned char*)state)[j];
    }
}

typedef union {
    unsigned char bytes[32];
    unsigned int words[8];
} SaltUnion;

__constant__ unsigned int mixConstant = 0x9e3779b9;

__constant__ unsigned char CONTRACT_BYTECODE_HASH[32] = {
    0x7f, 0x69, 0x47, 0x6e, 0xdf, 0xc9, 0x1a, 0x31,
    0x54, 0xb9, 0x06, 0xb2, 0x7f, 0xfc, 0x9d, 0xb0,
    0x85, 0x6c, 0x3b, 0x73, 0x27, 0xc8, 0x97, 0xef,
    0xe8, 0x25, 0x3a, 0xa4, 0xe7, 0x64, 0x40, 0x74
};

extern "C" __global__ void vanity_search(
    const unsigned char* deployer,
    const unsigned char* prefix,
    const unsigned int* prefix_len,
    const unsigned char* ns,
    const unsigned int* ns_len,
    const unsigned char* initial_salt,
    unsigned char* result_salt,
    int* found
) {
    int tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (atomicOr(found, 0) != 0) {
        return;
    }

    SaltUnion salt;

    for (unsigned int i = 0; i < 32; i++) {
        salt.bytes[i] = initial_salt[i];
    }
    for (unsigned int j = 0; j < 8; j++) {
        salt.words[j] ^= tid;
        salt.words[j] *= mixConstant;
    }

    // First compute namespaced salt
    unsigned char input[256];
    unsigned int input_len = 0;
    unsigned char final_salt[32];

    for (unsigned int i = 0; i < 32; i++) {
        final_salt[i] = salt.bytes[i];
    }

    // // Then hash with deployer
    // input_len = 0;
    // for (unsigned int i = 0; i < 20; i++) {
    //     input[input_len++] = deployer[i];
    // }
    // for (unsigned int i = 0; i < 32; i++) {
    //     input[input_len++] = final_salt[i];
    // }
    // keccak256(input, final_salt, input_len);

    // Compute CREATE2 address for proxy
    input_len = 0;
    input[input_len++] = 0xff;  // CREATE2 prefix
    for (unsigned int i = 0; i < 20; i++) {
        input[input_len++] = deployer[i];
    }
    for (unsigned int i = 0; i < 32; i++) {
        input[input_len++] = final_salt[i];
    }
    for (unsigned int i = 0; i < 32; i++) {
        input[input_len++] = CONTRACT_BYTECODE_HASH[i];
    }
    keccak256(input, final_salt, input_len);

    // // Finally compute CREATE address for contract
    // input_len = 0;
    // input[input_len++] = 0xd6;  // RLP prefix
    // input[input_len++] = 0x94;  // Address marker
    // for (unsigned int i = 0; i < 20; i++) {
    //     input[input_len++] = final_salt[12 + i];
    // }
    // input[input_len++] = 0x01;  // Nonce
    // keccak256(input, final_salt, input_len);

    // Compare the hex values with the prefix
    bool matches = true;
    for (unsigned int i = 0; i < *prefix_len && matches; i++) {
        // Get the byte from the address and extract the correct nibble
        unsigned int byte_index = i / 2;
        unsigned int nibble_index = i % 2;
        unsigned char address_byte = final_salt[12 + byte_index];
        unsigned char address_nibble;
        if (nibble_index == 0) {
            address_nibble = (address_byte >> 4) & 0xF;  // high nibble
        } else {
            address_nibble = address_byte & 0xF;  // low nibble
        }
        matches = matches && (prefix[i] == address_nibble);
    }
    
    if (matches) {
        if (atomicCAS(found, 0, 1) == 0) {
            for (unsigned int i = 0; i < 32; i++) {
                result_salt[i] = salt.bytes[i];
            }
            return;
        }
    }
} 