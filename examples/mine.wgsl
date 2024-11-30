struct Input {
    prefix: array<u32, 16>,
    nonce: vec2<u32>,
    difficulty: array<u32, 8>,
}

@group(0) @binding(0) var<storage, read> input: Input;
@group(0) @binding(1) var<storage, read_write> output_found: atomic<u32>;
@group(0) @binding(2) var<storage, read_write> output_nonce: vec2<u32>;

const prefix_u32_size: u32 = 16;
const hash_u32_size: u32 = 256 / 32;
const queue_size: u32 = 34;

@compute @workgroup_size(16 * 16 * 2, 1) fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (atomicLoad(&output_found) >= 1) {
        return;
    }

    var queue: array<u32, 34>;

    var nonce: vec2<u32>;
    nonce[0] = input.nonce[0] + global_id[0];
    nonce[1] = input.nonce[1] + global_id[1];

    absorb_prefix(&queue);

    let is_found = hash_with_nonce(&queue, nonce);
    if (is_found) {
        if (atomicLoad(&output_found) == 0) {
            atomicStore(&output_found, 1u);
            output_nonce = nonce;
            return;
        } else {
            return;
        }
    }
}

fn absorb_prefix(queue: ptr<function, array<u32, 34>>) {
    for (var i = 0u; i < prefix_u32_size; i++) {
        (*queue)[i] = input.prefix[i];
    }
    (*queue)[prefix_u32_size + 2] = (*queue)[prefix_u32_size + 2] | 0x01;
    (*queue)[queue_size - 1] = (*queue)[queue_size - 1] | 0x80000000;
}

fn hash_with_nonce(queue: ptr<function, array<u32, 34>>, nonce: vec2<u32>) -> bool {
    var state: array<u32, 50>;

    (*queue)[prefix_u32_size] = nonce[0];
    (*queue)[prefix_u32_size + 1] = nonce[1];

    for (var i = 0u; i < queue_size; i++) {
        state[i] = (*queue)[i];
    }

    permutation(&state);

    for (var i = 0u; i < 8; i++) {
        let lhs_u32 = state[i];
        let rhs_u32 = input.difficulty[i];
        for (var j = 0u; j < 32; j += 8u) {
            let lhs_byte = lhs_u32 >> j & 0xff;
            let rhs_byte = rhs_u32 >> j & 0xff;
            if (lhs_byte > rhs_byte) {
                return false;
            }
        }
    }

    return true;
}

fn permutation(state: ptr<function, array<u32, 50>>) {
    for (var r = 0u; r < 24; r++) {
        var c: array<u32, 10>;
        var w: vec2<u32>;
        // Theta
        for (var i = 0u; i < 10; i++) {
            c[i] = (*state)[i] ^ (*state)[i + 10] ^ (*state)[i + 20] ^ (*state)[i + 30] ^ (*state)[i + 40];
        }
        for (var x = 0u; x < 5; x++) {
            w = vec2<u32>(c[((x + 1) % 5) * 2], c[((x + 1) % 5) * 2 + 1]);
            let t = w;
            w = vec2<u32>(t[0] << 1 | t[1] >> 31, t[1] << 1 | t[0] >> 31);
            let d = vec2<u32>(c[(x + 4) % 5 * 2] ^ w[0], c[(x + 4) % 5 * 2 + 1] ^ w[1]);
            for (var y = 0u; y < 25; y += 5u) {
                (*state)[(x + y) * 2] ^= d[0];
                (*state)[(x + y) * 2 + 1] ^= d[1];
            }
        }
        // Rho and pi
        w = vec2<u32>((*state)[2], (*state)[3]);
        for (var i = 0u; i < 24; i++) {
            let p = P[i];
            let c = vec2<u32>((*state)[p * 2], (*state)[p * 2 + 1]);
            let r = (i + 1) * (i + 2) / 2 % 64;
            let t = w;
            if (r < 32) {
                w[0] = t[0] << r | t[1] >> (32 - r);
                w[1] = t[1] << r | t[0] >> (32 - r);
            } else {
                w[0] = t[1] << r | t[0] >> (32 - r);
                w[1] = t[0] << r | t[1] >> (32 - r);
            }
            (*state)[p * 2] = w[0];
            (*state)[p * 2 + 1] = w[1];
            w = c;
        }
        // Chi
        for (var y = 0u; y < 25; y += 5u) {
            for (var x = 0u; x < 5; x++) {
                c[x * 2] = (*state)[(x + y) * 2];
                c[x * 2 + 1] = (*state)[(x + y) * 2 + 1];
            }
            for (var x = 0u; x < 5; x++) {
                let xy = (x + y) * 2;
                let x1 = (x + 1) % 5 * 2;
                let x2 = (x + 2) % 5 * 2;
                (*state)[xy] ^= ~c[x1] & c[x2];
                (*state)[xy + 1] ^= ~c[x1 + 1] & c[x2 + 1];
            }
        }
        // Iota
        (*state)[0] ^= RC[r * 2];
        (*state)[1] ^= RC[r * 2 + 1];
    }
}

var<private> P: array<u32, 24> = array<u32, 24>(10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1);
var<private> RC: array<u32, 48> = array<u32, 48>(
    0x00000001, 0x00000000,
    0x00008082, 0x00000000,
    0x0000808a, 0x80000000,
    0x80008000, 0x80000000,
    0x0000808b, 0x00000000,
    0x80000001, 0x00000000,
    0x80008081, 0x80000000,
    0x00008009, 0x80000000,
    0x0000008a, 0x00000000,
    0x00000088, 0x00000000,
    0x80008009, 0x00000000,
    0x8000000a, 0x00000000,
    0x8000808b, 0x00000000,
    0x0000008b, 0x80000000,
    0x00008089, 0x80000000,
    0x00008003, 0x80000000,
    0x00008002, 0x80000000,
    0x00000080, 0x80000000,
    0x0000800a, 0x00000000,
    0x8000000a, 0x80000000,
    0x80008081, 0x80000000,
    0x00008080, 0x80000000,
    0x80000001, 0x00000000,
    0x80008008, 0x80000000
);
