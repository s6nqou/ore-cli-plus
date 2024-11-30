@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read> input_size: u32;
@group(0) @binding(2) var<storage, read_write> output: array<u32, 8>;

const digest_byte_size: u32 = 256 / 8;
const digest_u32_size = digest_byte_size / 4;

fn round(state: ptr<function, array<u32, 50>>, index: u32) {
    var P: array<u32, 24> = array<u32, 24> (10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1);
    var R: array<u32, 24> = array<u32, 24> (1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44);
    var RC: array<u32, 48> = array<u32, 48> (
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

    var c: array<u32, 10>;
    var d: array<u32, 10>;
    var w: array<u32, 2>;
    // Theta
    for (var i: u32 = 0; i < 10; i++) {
        c[i] = (*state)[i] ^ (*state)[i + 10] ^ (*state)[i + 20] ^ (*state)[i + 30] ^ (*state)[i + 40];
    }
    for (var x: u32 = 0; x < 5; x++) {
        w[0] = c[((x + 1) % 5) * 2];
        w[1] = c[((x + 1) % 5) * 2 + 1];
        let l = w[0];
        let h = w[1];
        w[0] = l << 1 | h >> 31;
        w[1] = h << 1 | l >> 31;
        d[x * 2] = c[(x + 4) % 5 * 2] ^ w[0];
        d[x * 2 + 1] = c[(x + 4) % 5 * 2 + 1] ^ w[1];
        for (var y: u32 = 0; y < 25; y = y + 5) {
            (*state)[(x + y) * 2] ^= d[x * 2];
            (*state)[(x + y) * 2 + 1] ^= d[x * 2 + 1];
        }
    }
    // Rho and pi
    w[0] = (*state)[2];
    w[1] = (*state)[3];
    for (var i: u32 = 0; i < 24; i++) {
        let p = P[i];
        let r = R[i];
        c[0] = (*state)[p * 2];
        c[1] = (*state)[p * 2 + 1];
        let l = w[0];
        let h = w[1];
        let ri: u32 = 32 - r;
        var j = select(1, 0, r < 32);
        w[j] = l << r | h >> ri;
        w[(j + 1) % 2] = h << r | l >> ri;
        (*state)[p * 2] = w[0];
        (*state)[p * 2 + 1] = w[1];
        w[0] = c[0];
        w[1] = c[1];
    }
    // Chi
    for (var y: u32 = 0; y < 25; y = y + 5) {
        for (var x: u32 = 0; x < 5; x++) {
            c[x * 2] = (*state)[(x + y) * 2];
            c[x * 2 + 1] = (*state)[(x + y) * 2 + 1];
        }
        for (var x: u32 = 0; x < 5; x++) {
            let xy = (x + y) * 2;
            let x1 = (x + 1) % 5 * 2;
            let x2 = (x + 2) % 5 * 2;
            (*state)[xy] ^= ~c[x1] & c[x2];
            (*state)[xy + 1] ^= ~c[x1 + 1] & c[x2 + 1];
        }
    }
    // Iota
    (*state)[0] ^= RC[index * 2];
    (*state)[1] ^= RC[index * 2 + 1];
}

fn hash() {
    var state: array<u32, 50>;
    var queue: array<u32, 34>;
    var queue_size: u32 = 34;
    var queue_offset: u32 = 0;
    var zero: u32 = 0;

    // Absorb phase
    for (var i: u32 = 0; i < arrayLength(&input); i++) {
        queue[queue_offset] = input[i];
        queue_offset++;
        if (queue_offset >= queue_size) {
            for (var j: u32 = 0; j < queue_size; j++) {
                state[j] ^= queue[j];
            }
            for (var r: u32 = 0; r < 24; r++) {
                round(&state, r);
            }
            queue_offset = zero;
        }
    }
    // Squeeze phase
    for (var i: u32 = queue_offset; i < queue_size; i++) {
        queue[i] = zero;
    }
    var input_offset: u32 = (input_size + 4) % 4;
    var padding: u32 = 0x01;
    if (input_offset > 0) {
        queue[queue_offset - 1] = queue[queue_offset - 1] | (padding << input_offset * 8);
    } else {
        queue[queue_offset] = queue[queue_offset] | padding;
    }
    queue[queue_size - 1] = queue[queue_size - 1] | 0x80000000;
    for (var j: u32 = 0; j < queue_size; j++) {
        state[j] ^= queue[j];
    }
    for (var r: u32 = 0; r < 24; r++) {
        round(&state, r);
    }
    // Output
    for (var o: u32 = 0; o < digest_u32_size; o++) {
        output[o] = state[o];
    }
}

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    hash();
}
