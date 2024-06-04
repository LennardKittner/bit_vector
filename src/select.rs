use crate::BitVector;
use crate::select::Block::{LargeBlock, SmallBlock};
use crate::select::SuperBlock::{LargeSuperBlock, SmallSuperBlock};

pub struct SelectAccelerator {
    super_block_offsets: Vec<usize>,
    super_blocks: Vec<SuperBlock>,
    zeros_per_super_block: usize,
    large_super_block_size: usize,
    large_block_size: usize
}

enum SuperBlock {
    LargeSuperBlock{
        select_table: Vec<usize>
    },
    SmallSuperBlock{
        blocks: Vec<Block>
    },
}

enum Block {
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
        self.zeros_per_super_block = bit_vector.len().ilog2().pow(2) as usize;
        self.large_super_block_size = self.zeros_per_super_block.pow(2);
        self.large_block_size = (bit_vector.len().ilog2() as f64).sqrt() as usize;
        
        self.super_block_offsets.push(0);
        let mut zeroes = 0;
        for i in 0..bit_vector.len() {
            zeroes += 1 - bit_vector.access(i);
            if zeroes != self.zeros_per_super_block {
                continue;
            }
            zeroes = 0;
            self.super_block_offsets.push(i);
            let next_block = self.super_block_offsets.len()-1;
            if self.super_block_offsets[next_block] - self.super_block_offsets[next_block-1] >= self.large_super_block_size {
                self.super_blocks.push(self.create_large_super_block(bit_vector, next_block-1));
            } else {
                self.create_small_super_block(bit_vector, next_block-1);
                self.super_blocks.push(SmallSuperBlock { blocks: vec![] });
            }
        }

        self.super_block_offsets.push(bit_vector.len());
        let next_block = self.super_block_offsets.len()-1;
        if self.super_block_offsets[next_block] - self.super_block_offsets[next_block-1] >= self.large_super_block_size {
            self.super_blocks.push(self.create_large_super_block(bit_vector, next_block-1));
        } else {
            self.create_small_super_block(bit_vector, next_block-1);
            self.super_blocks.push(SmallSuperBlock { blocks: vec![] });
        }
        self.super_block_offsets.pop();
    }

    #[inline]
    fn create_large_super_block(&self, bit_vector: &BitVector, block_index: usize) -> SuperBlock {
        let mut select_table = Vec::new();
        for j in self.super_block_offsets[block_index]..self.super_block_offsets[block_index+1] {
            if bit_vector.access(j) == 0 {
                select_table.push(j - self.super_block_offsets[block_index]);
            }
        }
        LargeSuperBlock{ select_table }
    }

    #[inline]
    fn create_small_super_block(&self, bit_vector: &BitVector, block_index: usize) {
        // let mut select_block_offsets = Vec::new();
        // let mut select_blocks = Vec::new();
        // select_block_offsets.push(0);
        // let mut zeroes;
        // for j in self.super_block_offsets[block_index]..self.super_block_offsets[block_index+1] {
        //     zeroes = 1 - bit_vector.access(j);
        //     if zeroes == self.large_block_size {
        //         zeroes = 0;
        //         select_block_offsets.push(j);
        //         let next_block = select_block_offsets.len();
        //         if select_block_offsets[next_block] - select_block_offsets[next_block - 1] >= self.large_block_size {
        //             // large block
        //             let mut select_table = Vec::new();
        //             for j in self.super_block_offsets[next_block - 1]..self.super_block_offsets[next_block] {
        //                 if bit_vector.access(j) == 0 {
        //                     select_table.push(j);
        //                 }
        //             }
        //             select_blocks.push(LargeBlock { select_table });
        //         } else {
        //             select_blocks.push(SmallBlock);
        //         }
        //     }
        // }
    }

    #[inline]
    pub fn select(&self, bit: bool, index: usize, bit_vector: &BitVector) -> usize {
        todo!()
    }
}

#[cfg(test)]
pub mod test {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use crate::BitVector;
    use crate::select::SuperBlock;

    #[test]
    fn test_init_large_blocks() {
        let mut data = String::new();
        let mut rng = ChaCha8Rng::seed_from_u64(1234567);
        for _i in 0..1000000 {
            if rng.gen_range(0..=1000) == 0 {
                data += "0";
            } else {
                data += "1";
            }
        }
        let mut bit_vector = BitVector::load_from_string(&data);
        bit_vector.init_select_structures();

        let select_accelerator = bit_vector.select_accelerator.as_ref().unwrap();

        let mut zeroes = 0;
        let mut block_index = 0;
        let mut block_start = 0;
        for i in 0..bit_vector.len() {
            zeroes += 1 - bit_vector.access(i);
            if zeroes != select_accelerator.zeros_per_super_block && i != bit_vector.len()-1 {
                continue;
            }
            if i - block_start > select_accelerator.large_super_block_size {
                let mut current_zero = 0;
                if let SuperBlock::LargeSuperBlock { select_table } = &select_accelerator.super_blocks[block_index] {
                    println!("alsökdjföl");
                    for j in block_start..i {
                        if bit_vector.access(j) == 0 {
                            assert_eq!(select_table[current_zero], j - block_start);
                            current_zero += 1;
                        }
                    }
                }
            }
            block_start = i;
            block_index += 1;
            zeroes = 0;
        }
    }
}