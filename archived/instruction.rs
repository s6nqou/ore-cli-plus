use solana_sdk::{compute_budget::ComputeBudgetInstruction, instruction::Instruction};

pub struct OreInstruction {
    pub compute_unit_limit: Option<u32>,
    pub compute_unit_price: Option<u64>,
    pub instruction: Instruction,
}

impl OreInstruction {
    pub fn new(
        compute_unit_limit: Option<u32>,
        compute_unit_price: Option<u64>,
        instruction: Instruction,
    ) -> Self {
        OreInstruction {
            compute_unit_limit,
            compute_unit_price,
            instruction,
        }
    }

    pub fn to_vec(&self) -> Vec<Instruction> {
        let mut vec = vec![];
        if let Some(compute_unit_limit) = self.compute_unit_limit {
            vec.push(ComputeBudgetInstruction::set_compute_unit_limit(
                compute_unit_limit,
            ));
        }
        if let Some(compute_unit_price) = self.compute_unit_price {
            vec.push(ComputeBudgetInstruction::set_compute_unit_price(
                compute_unit_price,
            ));
        }
        vec.push(self.instruction.clone());

        vec
    }
}
