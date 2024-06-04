use crate::BitVector;
use crate::select::SelectBlock::{LargeBlock, SmallBlock};
use crate::select::SelectSuperBlock::LargeSuperBlock;

pub struct SelectAccelerator {
    super_block_offsets: Vec<usize>,
    super_blocks: Vec<SelectSuperBlock>,
    zeros_per_super_block: usize,
    large_super_block_size: usize,
    large_block_size: usize
}

enum SelectSuperBlock {
    LargeSuperBlock{
        select_table: Vec<usize>
    },
    SmallSuperBlock{
        blocks: Vec<SelectBlock>
    },
}

enum SelectBlock {
    LargeBlock{
        select_table: Vec<usize>
    },
    SmallBlock
}


impl SelectAccelerator {
    
    pub fn new() -> Self {
        SelectAccelerator {
            super_block_offsets: Vec::new(),
            super_blocks: Vec::new(),
            zeros_per_super_block: 0,
            large_super_block_size: 0,
            large_block_size: 0,
        }
    }

    pub fn init(&mut self, bit_vector: &BitVector) {
        self.zeros_per_super_block = bit_vector.len.ilog2().pow(2) as usize;
        self.large_super_block_size = self.zeros_per_super_block.pow(2);
        self.large_block_size = (bit_vector.len.ilog2() as f64).sqrt() as usize;
        self.super_block_offsets.push(0);
        let mut zeroes = 0;
        for i in 0..bit_vector.len {
            zeroes = 1 - bit_vector.access(i);
            if zeroes == self.zeros_per_super_block {
                zeroes = 0;
                self.super_block_offsets.push(i);
                let next_block = self.super_block_offsets.len();
                if self.super_block_offsets[next_block] - self.super_block_offsets[next_block-1] >= self.large_super_block_size {
                    // large super block
                    let mut select_table = Vec::new();
                    for j in self.super_block_offsets[next_block-1]..self.super_block_offsets[next_block] {
                        if bit_vector.access(j) == 0 {
                            select_table.push(j);
                        }
                    }
                    self.super_blocks.push(LargeSuperBlock{ select_table })
                } else {
                    // small super block
                    let mut select_block_offsets = Vec::new();
                    let mut select_blocks = Vec::new();
                    select_block_offsets.push(0);
                    for j in self.super_block_offsets[next_block-1]..self.super_block_offsets[next_block] {
                        zeroes = 1 - bit_vector.access(j);
                       if zeroes == self.large_block_size {
                           zeroes = 0;
                           select_block_offsets.push(j);
                           let next_block = select_block_offsets.len();
                            if select_block_offsets[next_block] - select_block_offsets[next_block-1] >= self.large_block_size {
                                // large block
                                let mut select_table = Vec::new();
                                for j in self.super_block_offsets[next_block-1]..self.super_block_offsets[next_block] {
                                    if bit_vector.access(j) == 0 {
                                        select_table.push(j);
                                    }
                                }
                                select_blocks.push( LargeBlock{ select_table });
                            } else {
                                select_blocks.push(SmallBlock);
                            }
                       }
                    }
                }
            }
        }
    }

    #[inline]
    pub fn select(&self, bit: bool, index: usize, bit_vector: &BitVector) -> usize {
        todo!()
    }
}