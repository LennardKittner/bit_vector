use std::cmp::min;
use std::mem::size_of;
use crate::BitVector;
use crate::select::Block::{LargeBlock, SmallBlock};
use crate::select::SuperBlock::{LargeSuperBlock, SmallSuperBlock};
use crate::select_table::select_with_table;

pub struct SelectAccelerator<const BIT: bool> {
    super_blocks: Vec<SuperBlock<BIT>>,
    zeros_per_super_block: usize,
    zeros_per_block: usize,
    large_super_block_size: usize,
    large_block_size: usize
}

// The block and super block types are smaller but the alignment will increase the effective size to the same as the enums.
// #[repr(u8)]
// enum BlockType {
//     Large = 1,
//     Small = 2,
// }
//
// #[repr(C)]
// union SuperBlockData<const BIT: bool> {
//     select_table: ManuallyDrop<Vec<usize>>,
//     blocks: ManuallyDrop<Vec<Block<BIT>>>,
// }
//
// #[repr(packed)]
// struct SuperBlock<const BIT: bool> { // Size 25
//     t: BlockType,
//     d: SuperBlockData<BIT>,
// }
//
// #[repr(C)]
// union BlockData<const BIT: bool> {
//     select_table: ManuallyDrop<Box<Vec<usize>>>,
//     offset: usize,
// }
//
// #[repr(packed)]
// struct Block<const BIT: bool> { // Size 9
//     t: BlockType,
//     d: BlockData<BIT>,
// }

enum SuperBlock<const BIT: bool> { // Size 32
    LargeSuperBlock{
        select_table: Vec<usize>
    },
    SmallSuperBlock{
        blocks: Vec<Block<BIT>>
    },
}

impl<const BIT: bool> SuperBlock<BIT> {
    fn get_size(&self) -> usize {
        match self {
            LargeSuperBlock { select_table } => size_of::<SuperBlock<BIT>>() + select_table.capacity() * size_of::<usize>(),
            SmallSuperBlock { blocks } => size_of::<SuperBlock<BIT>>() + blocks.iter().map(Block::get_size).sum::<usize>()
        }
    }
}

enum Block<const BIT: bool> { // Size 16
    LargeBlock{
        // The size of the enum is dictated by the largest variant using Box makes the variant smaller
        select_table: Box<Vec<usize>>
    },
    // Block has size of usize anyway, so we can just store the offset directly
    SmallBlock{
        offset: usize,
    }
}

impl<const BIT: bool> Block<BIT> {
    fn get_size(&self) -> usize {
        match self {
            LargeBlock { select_table } => size_of::<Block<BIT>>() + size_of::<Vec<usize>>() + select_table.capacity() * size_of::<usize>(),
            SmallBlock { .. } => size_of::<Block<BIT>>(),
        }
    }
}

impl<const BIT: bool> SelectAccelerator<BIT> {
    pub fn new() -> SelectAccelerator<BIT> {
        SelectAccelerator {
            super_blocks: Vec::new(),
            zeros_per_super_block: 0,
            zeros_per_block: 0,
            large_super_block_size: 0,
            large_block_size: 0,
        }
    }

    pub fn get_size(&self) -> usize {
        let mut table_space = 0;
        #[cfg(feature = "USE_SELECT_TABLE")] {
            table_space = 2 * size_of::<[[u8; 8]; 256]>();
        }
        size_of::<SelectAccelerator<BIT>>()
        + self.super_blocks.iter().map(SuperBlock::get_size).sum::<usize>()
        + table_space
    }

    pub fn init(&mut self, bit_vector: &BitVector) {
        self.zeros_per_super_block = bit_vector.len().ilog2().pow(2) as usize;
        self.large_super_block_size = self.zeros_per_super_block.pow(2);
        self.large_block_size = bit_vector.len().ilog2() as usize;
        self.zeros_per_block = (self.large_block_size as f64).sqrt() as usize;
        let mut current_super_block_offset = 0;
        let mut next_super_block_offset;

        let mut zeroes = 0;
        for i in 0..bit_vector.len() {
            zeroes += if BIT { bit_vector.access(i) } else { 1 - bit_vector.access(i) };
            if zeroes != self.zeros_per_super_block && i != bit_vector.len()-1 {
                continue;
            }
            zeroes = 0;
            next_super_block_offset = i+1;
            if next_super_block_offset - current_super_block_offset >= self.large_super_block_size {
                self.super_blocks.push(self.create_large_super_block(bit_vector, current_super_block_offset, next_super_block_offset));
            } else {
                self.super_blocks.push(self.create_small_super_block(bit_vector, current_super_block_offset, next_super_block_offset));
            }
            current_super_block_offset = next_super_block_offset;
        }

        // This is faster than calculating the size of super_blocks in advance
        self.super_blocks.shrink_to_fit();
    }

    fn calc_select_table(bit_vector: &BitVector, start_index: usize, end_index: usize) -> Vec<usize> {
        let mut select_table = Vec::new();
        for j in start_index..end_index {
            if bit_vector.access(j) == if BIT { 1 } else { 0 } {
                // store index directly, so we don't have to sum over super blocks
                // we also don't have to store super block offsets anymore
                select_table.push(j);
            }
        }
        select_table.shrink_to_fit();
        select_table
    }

    #[inline]
    fn create_large_super_block(&self, bit_vector: &BitVector, super_block_start: usize, super_block_end: usize) -> SuperBlock<BIT> {
        LargeSuperBlock{ select_table: Self::calc_select_table(bit_vector, super_block_start, super_block_end) }
    }

    #[inline]
    fn create_small_super_block(&self, bit_vector: &BitVector, super_block_start: usize, super_block_end: usize) -> SuperBlock<BIT> {
        let mut blocks = Vec::new();
        let mut current_block_offset = super_block_start;
        let mut next_block_offset;

        let mut zeroes = 0;
        for j in super_block_start..super_block_end {
            zeroes += if BIT { bit_vector.access(j) } else { 1 - bit_vector.access(j) };
            if zeroes != self.zeros_per_block && j != super_block_end-1 {
                continue;
            }
            next_block_offset = j+1;
            if next_block_offset - current_block_offset >= self.large_block_size {
                // large block
                blocks.push(LargeBlock { select_table:  Box::new(Self::calc_select_table(bit_vector, current_block_offset, next_block_offset)) });
            } else {
                // small block
                blocks.push(SmallBlock { offset: current_block_offset });
            }
            zeroes = 0;
            current_block_offset = next_block_offset;
        }

        // remove last index which points to the next super block offset
        blocks.shrink_to_fit();
        SmallSuperBlock { blocks }
    }

    #[inline]
    pub fn select(&self, index: usize, bit_vector: &BitVector) -> usize {
        let super_block_index = index / self.zeros_per_super_block;
        match &self.super_blocks[super_block_index] {
            LargeSuperBlock{ select_table} => select_table[index],
            SmallSuperBlock{ blocks } => {
                let block_index = (index % self.zeros_per_super_block) / self.zeros_per_block;
                match &blocks[block_index] {
                    LargeBlock{ select_table} => select_table[(index % self.zeros_per_super_block) % self.zeros_per_block],
                    SmallBlock{ offset} => {
                        offset
                            + select_with_table(BIT, bit_vector.access_block(*offset) as usize, (index % self.zeros_per_super_block) % self.zeros_per_block).expect("No ith zero/one found in block")
                    }
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

        let select_accelerator_0 = bit_vector.select_accelerator_0.as_ref().unwrap();

        let mut zeroes = 0;
        let mut super_block_index = 0;
        let mut super_block_start = 0;
        for i in 0..bit_vector.len() {
            zeroes += 1 - bit_vector.access(i);
            if zeroes != select_accelerator_0.zeros_per_super_block && i != bit_vector.len()-1 {
                continue;
            }
            if i - super_block_start > select_accelerator_0.large_super_block_size {
                let mut current_zero = 0;
                if let SuperBlock::LargeSuperBlock { select_table } = &select_accelerator_0.super_blocks[super_block_index] {
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

        let select_accelerator_0 = bit_vector.select_accelerator_0.as_ref().unwrap();

        let mut zeroes = 0;
        let mut super_block_index = 0;
        let mut super_block_start = 0;
        for i in 0..bit_vector.len() {
            zeroes += 1 - bit_vector.access(i);
            if zeroes != select_accelerator_0.zeros_per_super_block && i != bit_vector.len()-1 {
                continue;
            }
            if i - super_block_start <= select_accelerator_0.large_super_block_size {
                let start_next_super_block = i+1;
                if let SuperBlock::SmallSuperBlock { blocks } = &select_accelerator_0.super_blocks[super_block_index] {
                    let mut block_index = 0;
                    let mut zeroes_in_block = 0;
                    let mut block_start = super_block_start;
                    for j in super_block_start..start_next_super_block {
                        zeroes_in_block += 1 - bit_vector.access(j);
                        if zeroes_in_block != select_accelerator_0.zeros_per_block && j != start_next_super_block -1 {
                            continue;
                        }
                        if j - block_start <= select_accelerator_0.large_block_size {
                            let start_next_block = j+1;
                            match &blocks[block_index] {
                                Block::LargeBlock { select_table } => {
                                    let mut current_zero = 0;
                                    for k in block_start..start_next_block {
                                        if bit_vector.access(k) == 0 {
                                            assert_eq!(select_table[current_zero], k);
                                            current_zero += 1;
                                        }
                                    }
                                },
                                Block::SmallBlock { offset } => {
                                    assert_eq!(*offset, block_start);
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