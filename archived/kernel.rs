#![cfg_attr(target_arch = "spirv", no_std)]

use spirv_std::arch::{atomic_load, atomic_store, IndexUnchecked};
use spirv_std::glam::UVec3;
use spirv_std::memory::{Scope, Semantics};
use spirv_std::spirv;

pub struct Input {
    prefix: Prefix,
    nonce: u64,
    difficulty: Hash,
}

const QUEUE_SIZE: usize = 17;
const PREFIX_SIZE: usize = 8;

type Queue = [u64; QUEUE_SIZE];
type Prefix = [u64; PREFIX_SIZE];
type State = [u64; 25];
type Hash = [u64; 4];

#[spirv(compute(threads(1, 1)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] global_id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &Input,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] output_found: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] output_nonce: &mut [u64],
) {
    unsafe {
        let found = atomic_load::<_, { Scope::Device as u32 }, { Semantics::UNIFORM_MEMORY.bits() }>(
            output_found.index_unchecked_mut(0),
        );

        if found > 0 {
            return;
        }
    }

    let mut queue: Queue = [0u64; 17];
    let mut hash: Hash = [0u64; 4];
    let nonce = input.nonce + global_id[0] as u64;

    absorb_prefix(&mut queue, &input.prefix);

    hash_with_nonce(&mut queue, nonce, &mut hash);

    if is_le(&hash, &input.difficulty) {
        unsafe {
            let found =
                atomic_load::<_, { Scope::Device as u32 }, { Semantics::UNIFORM_MEMORY.bits() }>(
                    output_found.index_unchecked_mut(0),
                );

            if found > 0 {
                return;
            }

            atomic_store::<_, { Scope::Device as u32 }, { Semantics::UNIFORM_MEMORY.bits() }>(
                output_found.index_unchecked_mut(0),
                1,
            );

            output_found[0] = 1;
            output_nonce[0] = 2;
        }
    }
}

fn hash_with_nonce(queue: &mut Queue, nonce: u64, output: &mut Hash) {
    let mut state: State = [0u64; 25];

    queue[QUEUE_SIZE] = nonce;

    for i in 0..17 {
        state[i] = queue[i];
    }

    permutate(&mut state);

    for i in 0..4 {
        output[i] = state[i];
    }
}

fn permutate(state: &mut State) {
    let mut c = [0u64; 5];

    for r in 0..24 {
        // Theta
        for i in 0..5 {
            c[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }
        for i in 0..5 {
            let t = c[(i + 4) % 5] ^ rot_l(c[(i + 1) % 5], 1);
            for j in 0..5 {
                state[i + j * 5] ^= t;
            }
        }
        // Rho and pi
        let mut temp = state[1];
        for i in 0..24 {
            let j = P[i] as usize;
            c[0] = state[j];
            state[j] = rot_l(temp, (i + 1) * (i + 2) / 2 % 64);
            temp = c[0];
        }
        // Chi
        for i in 0..5 {
            let mut t = [0u64; 5];
            for j in 0..5 {
                t[j] = state[i + j * 5];
            }
            for j in 0..5 {
                state[i + j * 5] ^= !t[(j + 1) % 5] & t[(j + 2) % 5];
            }
        }
        // Iota
        state[0] ^= RC[r];
    }
}

fn absorb_prefix(queue: &mut Queue, prefix: &Prefix) {
    for i in 0..8 {
        queue[i] = prefix[i];
    }
    queue[PREFIX_SIZE + 1] |= 0x01;
    queue[QUEUE_SIZE - 1] |= 0x8000000000000000;
}

fn rot_l(x: u64, n: usize) -> u64 {
    x << n | x >> (64 - n)
}

fn is_le(lhs: &Hash, rhs: &Hash) -> bool {
    for i in 0..4 {
        for j in 0..8 {
            let lhs_byte = lhs[i] >> (j * 8) & 0xff;
            let rhs_byte = rhs[i] >> (j * 8) & 0xff;
            if lhs_byte > rhs_byte {
                return false;
            }
        }
    }
    return true;
}

const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

const P: [u64; 24] = [
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
];
