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

constant int piln[24] = { 10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1 };

void keccak_f(thread ulong *state) {
    ulong C[5], D;
    for (int round = 0; round < 24; round++) {
        C[0] = state[0] ^ state[5] ^ state[10] ^ state[15] ^ state[20];
        C[1] = state[1] ^ state[6] ^ state[11] ^ state[16] ^ state[21];
        C[2] = state[2] ^ state[7] ^ state[12] ^ state[17] ^ state[22];
        C[3] = state[3] ^ state[8] ^ state[13] ^ state[18] ^ state[23];
        C[4] = state[4] ^ state[9] ^ state[14] ^ state[19] ^ state[24];

        D = C[4] ^ ROTL64(C[1], 1);
        state[0] ^= D;
        state[5] ^= D;
        state[10] ^= D;
        state[15] ^= D;
        state[20] ^= D;

        D = C[0] ^ ROTL64(C[2], 1);
        state[1] ^= D;
        state[6] ^= D;
        state[11] ^= D;
        state[16] ^= D;
        state[21] ^= D;

        D = C[1] ^ ROTL64(C[3], 1);
        state[2] ^= D;
        state[7] ^= D;
        state[12] ^= D;
        state[17] ^= D;
        state[22] ^= D;

        D = C[2] ^ ROTL64(C[4], 1);
        state[3] ^= D;
        state[8] ^= D;
        state[13] ^= D;
        state[18] ^= D;
        state[23] ^= D;

        D = C[3] ^ ROTL64(C[0], 1);
        state[4] ^= D;
        state[9] ^= D;
        state[14] ^= D;
        state[19] ^= D;
        state[24] ^= D;

        ulong tmp = state[1];
        int j0 = piln[0]; C[0] = state[j0]; state[j0] = ROTL64(tmp, 1); tmp = C[0];
        int j1 = piln[1]; C[0] = state[j1]; state[j1] = ROTL64(tmp, 3); tmp = C[0];
        int j2 = piln[2]; C[0] = state[j2]; state[j2] = ROTL64(tmp, 6); tmp = C[0];
        int j3 = piln[3]; C[0] = state[j3]; state[j3] = ROTL64(tmp, 10); tmp = C[0];
        int j4 = piln[4]; C[0] = state[j4]; state[j4] = ROTL64(tmp, 15); tmp = C[0];
        int j5 = piln[5]; C[0] = state[j5]; state[j5] = ROTL64(tmp, 21); tmp = C[0];
        int j6 = piln[6]; C[0] = state[j6]; state[j6] = ROTL64(tmp, 28); tmp = C[0];
        int j7 = piln[7]; C[0] = state[j7]; state[j7] = ROTL64(tmp, 36); tmp = C[0];
        int j8 = piln[8]; C[0] = state[j8]; state[j8] = ROTL64(tmp, 45); tmp = C[0];
        int j9 = piln[9]; C[0] = state[j9]; state[j9] = ROTL64(tmp, 55); tmp = C[0];
        int j10 = piln[10]; C[0] = state[j10]; state[j10] = ROTL64(tmp, 2); tmp = C[0];
        int j11 = piln[11]; C[0] = state[j11]; state[j11] = ROTL64(tmp, 14); tmp = C[0];
        int j12 = piln[12]; C[0] = state[j12]; state[j12] = ROTL64(tmp, 27); tmp = C[0];
        int j13 = piln[13]; C[0] = state[j13]; state[j13] = ROTL64(tmp, 41); tmp = C[0];
        int j14 = piln[14]; C[0] = state[j14]; state[j14] = ROTL64(tmp, 56); tmp = C[0];
        int j15 = piln[15]; C[0] = state[j15]; state[j15] = ROTL64(tmp, 8); tmp = C[0];
        int j16 = piln[16]; C[0] = state[j16]; state[j16] = ROTL64(tmp, 25); tmp = C[0];
        int j17 = piln[17]; C[0] = state[j17]; state[j17] = ROTL64(tmp, 43); tmp = C[0];
        int j18 = piln[18]; C[0] = state[j18]; state[j18] = ROTL64(tmp, 62); tmp = C[0];
        int j19 = piln[19]; C[0] = state[j19]; state[j19] = ROTL64(tmp, 18); tmp = C[0];
        int j20 = piln[20]; C[0] = state[j20]; state[j20] = ROTL64(tmp, 39); tmp = C[0];
        int j21 = piln[21]; C[0] = state[j21]; state[j21] = ROTL64(tmp, 61); tmp = C[0];
        int j22 = piln[22]; C[0] = state[j22]; state[j22] = ROTL64(tmp, 20); tmp = C[0];
        int j23 = piln[23]; C[0] = state[j23]; state[j23] = ROTL64(tmp, 44); tmp = C[0];

        ulong t0 = state[0], t1 = state[1], t2 = state[2], t3 = state[3], t4 = state[4];
        state[0] ^= ~t1 & t2;
        state[1] ^= ~t2 & t3;
        state[2] ^= ~t3 & t4;
        state[3] ^= ~t4 & t0;
        state[4] ^= ~t0 & t1;

        t0 = state[5], t1 = state[6], t2 = state[7], t3 = state[8], t4 = state[9];
        state[5] ^= ~t1 & t2;
        state[6] ^= ~t2 & t3;
        state[7] ^= ~t3 & t4;
        state[8] ^= ~t4 & t0;
        state[9] ^= ~t0 & t1;

        t0 = state[10], t1 = state[11], t2 = state[12], t3 = state[13], t4 = state[14];
        state[10] ^= ~t1 & t2;
        state[11] ^= ~t2 & t3;
        state[12] ^= ~t3 & t4;
        state[13] ^= ~t4 & t0;
        state[14] ^= ~t0 & t1;

        t0 = state[15], t1 = state[16], t2 = state[17], t3 = state[18], t4 = state[19];
        state[15] ^= ~t1 & t2;
        state[16] ^= ~t2 & t3;
        state[17] ^= ~t3 & t4;
        state[18] ^= ~t4 & t0;
        state[19] ^= ~t0 & t1;

        t0 = state[20], t1 = state[21], t2 = state[22], t3 = state[23], t4 = state[24];
        state[20] ^= ~t1 & t2;
        state[21] ^= ~t2 & t3;
        state[22] ^= ~t3 & t4;
        state[23] ^= ~t4 & t0;
        state[24] ^= ~t0 & t1;

        state[0] ^= RC[round];
    }
}

void keccak256(thread uchar *localInput, thread uchar *localOutput, thread uint inputLength)  {
    const uint rsize = 136; // 1088 bits (136 bytes) for Keccak-256
    thread ulong state[25] = {0};
    thread uint i = 0;
    while (i < inputLength) {
        if (i + rsize <= inputLength) {
            for (uint j = 0; j < rsize / 8; j++) {
                thread ulong block = 0;
                block |= (ulong)(localInput[i + j * 8 + 1]) << (8 * 1);
                block |= (ulong)(localInput[i + j * 8 + 2]) << (8 * 2);
                block |= (ulong)(localInput[i + j * 8 + 3]) << (8 * 3);
                block |= (ulong)(localInput[i + j * 8 + 4]) << (8 * 4);
                block |= (ulong)(localInput[i + j * 8 + 5]) << (8 * 5);
                block |= (ulong)(localInput[i + j * 8 + 6]) << (8 * 6);
                block |= (ulong)(localInput[i + j * 8 + 7]) << (8 * 7);
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
                thread ulong block = 0;
                block |= (ulong)(padded[j * 8 + 0]) << (8 * 0);
                block |= (ulong)(padded[j * 8 + 1]) << (8 * 1);
                block |= (ulong)(padded[j * 8 + 2]) << (8 * 2);
                block |= (ulong)(padded[j * 8 + 3]) << (8 * 3);
                block |= (ulong)(padded[j * 8 + 4]) << (8 * 4);
                block |= (ulong)(padded[j * 8 + 5]) << (8 * 5);
                block |= (ulong)(padded[j * 8 + 6]) << (8 * 6);
                block |= (ulong)(padded[j * 8 + 7]) << (8 * 7);
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
            thread ulong block = 0;
            block |= (ulong)(padded[j * 8 + 0]) << (8 * 0);
            block |= (ulong)(padded[j * 8 + 1]) << (8 * 1);
            block |= (ulong)(padded[j * 8 + 2]) << (8 * 2);
            block |= (ulong)(padded[j * 8 + 3]) << (8 * 3);
            block |= (ulong)(padded[j * 8 + 4]) << (8 * 4);
            block |= (ulong)(padded[j * 8 + 5]) << (8 * 5);
            block |= (ulong)(padded[j * 8 + 6]) << (8 * 6);
            block |= (ulong)(padded[j * 8 + 7]) << (8 * 7);
            state[j] ^= block;
        }
        keccak_f(state);
    }
    // Write the output
    localOutput[0] = (uchar)((state[0] >> (8 * 0)) & 0xFF);
    localOutput[1] = (uchar)((state[0] >> (8 * 1)) & 0xFF);
    localOutput[2] = (uchar)((state[0] >> (8 * 2)) & 0xFF);
    localOutput[3] = (uchar)((state[0] >> (8 * 3)) & 0xFF);
    localOutput[4] = (uchar)((state[0] >> (8 * 4)) & 0xFF);
    localOutput[5] = (uchar)((state[0] >> (8 * 5)) & 0xFF);
    localOutput[6] = (uchar)((state[0] >> (8 * 6)) & 0xFF);
    localOutput[7] = (uchar)((state[0] >> (8 * 7)) & 0xFF);
    localOutput[8] = (uchar)((state[1] >> (8 * 0)) & 0xFF);
    localOutput[9] = (uchar)((state[1] >> (8 * 1)) & 0xFF);
    localOutput[10] = (uchar)((state[1] >> (8 * 2)) & 0xFF);
    localOutput[11] = (uchar)((state[1] >> (8 * 3)) & 0xFF);
    localOutput[12] = (uchar)((state[1] >> (8 * 4)) & 0xFF);
    localOutput[13] = (uchar)((state[1] >> (8 * 5)) & 0xFF);
    localOutput[14] = (uchar)((state[1] >> (8 * 6)) & 0xFF);
    localOutput[15] = (uchar)((state[1] >> (8 * 7)) & 0xFF);
    localOutput[16] = (uchar)((state[2] >> (8 * 0)) & 0xFF);
    localOutput[17] = (uchar)((state[2] >> (8 * 1)) & 0xFF);
    localOutput[18] = (uchar)((state[2] >> (8 * 2)) & 0xFF);
    localOutput[19] = (uchar)((state[2] >> (8 * 3)) & 0xFF);
    localOutput[20] = (uchar)((state[2] >> (8 * 4)) & 0xFF);
    localOutput[21] = (uchar)((state[2] >> (8 * 5)) & 0xFF);
    localOutput[22] = (uchar)((state[2] >> (8 * 6)) & 0xFF);
    localOutput[23] = (uchar)((state[2] >> (8 * 7)) & 0xFF);
    localOutput[24] = (uchar)((state[3] >> (8 * 0)) & 0xFF);
    localOutput[25] = (uchar)((state[3] >> (8 * 1)) & 0xFF);
    localOutput[26] = (uchar)((state[3] >> (8 * 2)) & 0xFF);
    localOutput[27] = (uchar)((state[3] >> (8 * 3)) & 0xFF);
    localOutput[28] = (uchar)((state[3] >> (8 * 4)) & 0xFF);
    localOutput[29] = (uchar)((state[3] >> (8 * 5)) & 0xFF);
    localOutput[30] = (uchar)((state[3] >> (8 * 6)) & 0xFF);
    localOutput[31] = (uchar)((state[3] >> (8 * 7)) & 0xFF);
}

typedef union {
    uchar  bytes[32];
    uint   words[8];
} SaltUnion;

constant uint mixConstant = 0x9e3779b9;

kernel void vanity_search(
    const device uchar* deployer [[buffer(0)]],
    const device uchar* prefix [[buffer(1)]],
    const device uint* prefix_len [[buffer(2)]],
    const device uchar* bytecode_hash [[buffer(3)]],
    const device uchar* initial_salt [[buffer(4)]],
    device uchar* result_salt [[buffer(5)]],
    device atomic_bool* found [[buffer(6)]],
    uint tid [[thread_position_in_grid]]
) {
    if (atomic_load_explicit(found, memory_order_relaxed)) {
        return;
    }

    thread SaltUnion salt;

    for (uint i = 0; i < 32; i++) {
        salt.bytes[i] = initial_salt[i];
    }
    for (uint j = 0; j < 8; j++) {
        salt.words[j] ^= (tid + j * mixConstant);
        salt.words[j] *= mixConstant;
    }

    // First compute namespaced salt
    thread uchar input[256];
    thread uint input_len = 0;
    thread uchar final_salt[32];

    for (uint i = 0; i < 32; i++) {
        final_salt[i] = salt.bytes[i];
    }

    // Compute CREATE2 address for proxy
    input_len = 0;
    input[input_len++] = 0xff;  // CREATE2 prefix
    for (uint i = 0; i < 20; i++) {
        input[input_len++] = deployer[i];
    }
    for (uint i = 0; i < 32; i++) {
        input[input_len++] = final_salt[i];
    }
    for (uint i = 0; i < 32; i++) {
        input[input_len++] = bytecode_hash[i];
    }
    keccak256(input, final_salt, input_len);

    // Compare the hex values with the prefix
    bool matches = true;
    for (uint i = 0; i < *prefix_len && matches; i++) {
        // Get the byte from the address and extract the correct nibble
        uint byte_index = i / 2;
        uint nibble_index = i % 2;
        uchar address_byte = final_salt[12 + byte_index];
        uchar address_nibble;
        if (nibble_index == 0) {
            address_nibble = (address_byte >> 4) & 0xF;  // high nibble
        } else {
            address_nibble = address_byte & 0xF;  // low nibble
        }
        matches = matches && (prefix[i] == address_nibble);
    }
    
    if (matches) {
        bool expected = false;
        if (atomic_compare_exchange_weak_explicit(found, &expected, true, memory_order_relaxed, memory_order_relaxed)) {
            for (uint i = 0; i < 32; i++) {
                result_salt[i] = salt.bytes[i];
            }
            return;
        }
    }

    // atomic_fetch_add_explicit(debug_counter, 1, memory_order_relaxed);
}