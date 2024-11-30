use std::{collections::VecDeque, sync::mpsc};

use solana_sdk::keccak::{hashv, Hash};
use threadpool::ThreadPool;

use crate::mine::{Miner, Work};

pub struct Pipeline<'a> {
    miners: &'a Vec<Miner>,
    mine_threads: ThreadPool,
    mine_deque: VecDeque<Work>,
}

impl<'a> Pipeline<'a> {
    pub fn new(mine_workers: usize, miners: &'a Vec<Miner>) -> Self {
        let mine_threads = ThreadPool::new(mine_workers);
        let mine_deque: VecDeque<Work> = VecDeque::new();

        Pipeline {
            miners,
            mine_threads,
            mine_deque,
        }
    }

    pub fn run(&mut self) {
        let (sender, receiver) = mpsc::channel::<Work>();
        loop {
            if self.mine_deque.len() < 10 {}
        }
    }

}
