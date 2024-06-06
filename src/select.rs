use crate::BitVector;
use crate::select::Block::{LargeBlock, SmallBlock};
use crate::select::SuperBlock::{LargeSuperBlock, SmallSuperBlock};
use crate::select_table::select_with_table;

pub struct SelectAccelerator {
    super_block_offsets: Vec<usize>,
    super_blocks: Vec<SuperBlock>,
    zeros_per_super_block: usize,
    zeros_per_block: usize,
    large_super_block_size: usize,
    large_block_size: usize
}

enum SuperBlock {
    LargeSuperBlock{
        select_table: Vec<usize>
    },
    SmallSuperBlock{
        block_offsets: Vec<usize>,
        blocks: Vec<Block>
    },
}

enum Block {
    LargeBlock{
        select_table: Vec<usize>
    },
    // Block has size of usize anyway so we can just store the data directly
    SmallBlock{
        data: usize
    }
}


impl SelectAccelerator {
    
    pub fn new() -> Self {
        SelectAccelerator {
            super_block_offsets: Vec::new(),
            super_blocks: Vec::new(),
            zeros_per_super_block: 0,
            zeros_per_block: 0,
            large_super_block_size: 0,
            large_block_size: 0
        }
    }

    pub fn init(&mut self, bit_vector: &BitVector) {
        self.zeros_per_super_block = bit_vector.len().ilog2().pow(2) as usize;
        self.large_super_block_size = self.zeros_per_super_block.pow(2);
        self.large_block_size = bit_vector.len().ilog2() as usize;
        self.zeros_per_block = (self.large_block_size as f64).sqrt() as usize;

        self.super_block_offsets.push(0);
        let mut zeroes = 0;
        for i in 0..bit_vector.len() {
            zeroes += 1 - bit_vector.access(i);
            if zeroes != self.zeros_per_super_block && i != bit_vector.len()-1 {
                continue;
            }
            zeroes = 0;
            self.super_block_offsets.push(i+1);
            let next_block = self.super_block_offsets.len()-1;
            if self.super_block_offsets[next_block] - self.super_block_offsets[next_block-1] >= self.large_super_block_size {
                self.super_blocks.push(self.create_large_super_block(bit_vector, next_block-1));
            } else {
                self.super_blocks.push(self.create_small_super_block(bit_vector, next_block-1));
            }
        }

        // remove last offset because the last element contains bitvector.len()
        self.super_block_offsets.pop();
        self.super_block_offsets.shrink_to_fit();
        self.super_blocks.shrink_to_fit();
    }

    fn calc_select_table(bit_vector: &BitVector, start_index: usize, end_index: usize) -> Vec<usize> {
        let mut select_table = Vec::new();
        for j in start_index..end_index {
            if bit_vector.access(j) == 0 {
                // store index directly, so we don't have to sum over super blocks
                select_table.push(j);
            }
        }
        select_table.shrink_to_fit();
        select_table
    }

    #[inline]
    fn create_large_super_block(&self, bit_vector: &BitVector, block_index: usize) -> SuperBlock {
        LargeSuperBlock{ select_table: Self::calc_select_table(bit_vector, self.super_block_offsets[block_index], self.super_block_offsets[block_index+1]) }
    }

    #[inline]
    fn create_small_super_block(&self, bit_vector: &BitVector, block_index: usize) -> SuperBlock {
        let mut block_offsets = Vec::new();
        let mut blocks = Vec::new();

        block_offsets.push(self.super_block_offsets[block_index]);
        let mut zeroes = 0;
        for j in self.super_block_offsets[block_index]..self.super_block_offsets[block_index+1] {
            zeroes += 1 - bit_vector.access(j);
            if zeroes != self.zeros_per_block && j != self.super_block_offsets[block_index+1]-1 {
                continue;
            }
            block_offsets.push(j+1);
            let next_block = block_offsets.len()-1;
            if block_offsets[next_block] - block_offsets[next_block - 1] >= self.large_block_size {
                // large block
                blocks.push(LargeBlock { select_table:  Self::calc_select_table(bit_vector, block_offsets[next_block - 1], block_offsets[next_block]) });
            } else {
                // small block
                let mut data = 0;
                for k in block_offsets[next_block-1]..block_offsets[next_block] {
                    data |= bit_vector.access(k) << (k - block_offsets[next_block-1]);
                }
                blocks.push(SmallBlock { data });
            }
            zeroes = 0;
        }
        // remove last index which points to the next super block offset
        block_offsets.pop();
        SmallSuperBlock { block_offsets, blocks }
    }

    #[inline]
    pub fn select(&self, bit: bool, index: usize, bit_vector: &BitVector) -> usize {
        //todo: bit = true
        let super_block_index = index / self.zeros_per_super_block;
        match &self.super_blocks[super_block_index] {
            LargeSuperBlock{ select_table} => select_table[index],
            SmallSuperBlock{ block_offsets, blocks } => {
                let block_index = (index % self.zeros_per_super_block) / self.zeros_per_block;
                match &blocks[block_index] {
                    LargeBlock{ select_table} => select_table[(index % self.zeros_per_super_block) % self.zeros_per_block],
                    SmallBlock{ data} => block_offsets[block_index]
                        + select_with_table(*data, (index % self.zeros_per_super_block) % self.zeros_per_block).expect("No ith zero found in block")
                }
            },
        }
    }
}

#[cfg(test)]
pub mod test {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use crate::BitVector;
    use crate::select::{Block, SuperBlock};

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
        let mut super_block_index = 0;
        let mut super_block_start = 0;
        for i in 0..bit_vector.len() {
            zeroes += 1 - bit_vector.access(i);
            if zeroes != select_accelerator.zeros_per_super_block && i != bit_vector.len()-1 {
                continue;
            }
            assert_eq!(select_accelerator.super_block_offsets[super_block_index], super_block_start);
            if i - super_block_start > select_accelerator.large_super_block_size {
                let mut current_zero = 0;
                if let SuperBlock::LargeSuperBlock { select_table } = &select_accelerator.super_blocks[super_block_index] {
                    for j in super_block_start..i {
                        if bit_vector.access(j) == 0 {
                            assert_eq!(select_table[current_zero], j);
                            current_zero += 1;
                        }
                    }
                }
            }
            super_block_start = i+1;
            super_block_index += 1;
            zeroes = 0;
        }
    }

    #[ignore]
    #[test]
    fn test_init_small_blocks() {
        let mut data = String::new();
        let mut rng = ChaCha8Rng::seed_from_u64(1234567);
        for _ in 0..4096 {
            if rng.gen_range(0..=1) == 0 {
                data += "0";
            } else {
                data += "1";
            }
        }
        let mut bit_vector = BitVector::load_from_string(&data);
        bit_vector.init_select_structures();

        let select_accelerator = bit_vector.select_accelerator.as_ref().unwrap();

        let mut zeroes = 0;
        let mut super_block_index = 0;
        let mut super_block_start = 0;
        for i in 0..bit_vector.len() {
            zeroes += 1 - bit_vector.access(i);
            if zeroes != select_accelerator.zeros_per_super_block && i != bit_vector.len()-1 {
                continue;
            }
            assert_eq!(select_accelerator.super_block_offsets[super_block_index], super_block_start);
            if i - super_block_start <= select_accelerator.large_super_block_size {
                if let SuperBlock::SmallSuperBlock { block_offsets, blocks } = &select_accelerator.super_blocks[super_block_index] {
                    let mut block_index = 0;
                    let mut zeroes_in_block = 0;
                    let mut block_start = super_block_start;
                    for j in super_block_start..i {
                        zeroes_in_block += 1 - bit_vector.access(j);
                        if zeroes_in_block != select_accelerator.zeros_per_block && j != i-1 {
                            continue;
                        }
                        assert_eq!(block_offsets[block_index], block_start);
                        if j - block_start <= select_accelerator.large_block_size {
                            match &blocks[block_index] {
                                Block::LargeBlock { select_table } => {
                                    let mut current_zero = 0;
                                    for k in block_start..j {
                                        if bit_vector.access(k) == 0 {
                                            assert_eq!(select_table[current_zero], k);
                                            current_zero += 1;
                                        }
                                    }
                                },
                                Block::SmallBlock { data } => {
                                    let mut tmp = 0;
                                    for k in block_start..j {
                                        tmp |= bit_vector.access(k) << (k - block_start);
                                    }
                                    if tmp != *data {
                                        println!("alsökdjf")
                                    }
                                    //assert_eq!(data, &tmp)
                                }
                            }
                        }
                        block_start = j+1;
                        block_index += 1;
                        zeroes_in_block = 0;
                    }
                }
            }
            super_block_start = i+1;
            super_block_index += 1;
            zeroes = 0;
        }
    }
}