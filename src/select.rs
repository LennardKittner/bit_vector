use std::mem::size_of;
use std::ops::Range;
use crate::BitVector;
use crate::select::Block::{LargeBlock, SmallBlock};
use crate::select::SuperBlock::{LargeSuperBlock, SmallSuperBlock};
use crate::select_table::select_with_table;

/// An accelerator used to for select operations.
/// `BIT` specifies whether the accelerator should be used for zero = `false` or one = `true` select operations.
pub struct SelectAccelerator<const BIT: bool> {
    // Most variables and methods have zero in the name but if `BIT = true` it means one

    // holds the super blocks
    super_blocks: Vec<SuperBlock<BIT>>,
    // the number of zeroes/ones per super block
    zeros_per_super_block: usize,
    // the number of zeroes/ones per block
    zeros_per_block: usize,
    // the minimum size of a large super block
    large_super_block_size: usize,
    // the minimum size of a large block
    large_block_size: usize
}

// This is another possibility to store the block data.
// These block and super block types are smaller but the alignment will increase the effective size to the same as the enums.
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

/// A super block
enum SuperBlock<const BIT: bool> { // Size 32 = 24 from vec + 8 through alignment and enum discriminate
    /// Large / sparse super blocks store a lookup table
    /// Large super blocks are sparse enough, so it is more efficient to simply store a lookup table
    LargeSuperBlock{
        select_table: Vec<usize>
    },
    /// Small / dense super blocks store a vector of blocks
    /// There are too many small super blocks, so we can not simply store a lookup table and instead have to split the super block up further.
    SmallSuperBlock{
        blocks: Vec<Block<BIT>>
    },
}

impl<const BIT: bool> SuperBlock<BIT> {
    /// Get the size of the super block including space on the heap
    fn get_size(&self) -> usize {
        match self {
            LargeSuperBlock { select_table } => size_of::<SuperBlock<BIT>>() + select_table.capacity() * size_of::<usize>(),
            SmallSuperBlock { blocks } => size_of::<SuperBlock<BIT>>() + blocks.iter().map(Block::get_size).sum::<usize>()
        }
    }
}

/// A block
enum Block<const BIT: bool> { // Size 16 = 8 from usize / Box + 8 through alignment and enum discriminate
    /// Large / sparse blocks store a lookup table
    /// Large blocks are still sparse enough, so it is more efficient to simply store a lookup table
    LargeBlock{
        // The size of the enum is dictated by the largest variant using Box makes the variant smaller
        #[allow(clippy::box_collection)]
        select_table: Box<Vec<usize>>
    },
    /// Small / dense blocks store an offset to the block
    /// `Block` has size of 8 anyway, due to `LargeBlock`; so we can just store the offset directly.
    /// A small block is smaller than `large_block_size` <= `bit_vector.len().ilog2()` <= 64
    /// => we could store the block directly inside the variant.
    /// However, when executing a select operation we also need the offset, so we would have to store that too.
    /// Only storing the offset reduces the size of Block by 8 bytes at the price of having to read the block from the bit vector.
    /// We think this tradeoff is worth taking.
    SmallBlock{
        offset: usize,
    }
}

impl<const BIT: bool> Block<BIT> {
    /// Get the size of the block including space on the heap
    fn get_size(&self) -> usize {
        match self {
            LargeBlock { select_table } => size_of::<Block<BIT>>() + size_of::<Vec<usize>>() + select_table.capacity() * size_of::<usize>(),
            SmallBlock { .. } => size_of::<Block<BIT>>(),
        }
    }
}

impl<const BIT: bool> SelectAccelerator<BIT> {
    /// Creates an uninitialized select accelerator
    pub fn new() -> SelectAccelerator<BIT> {
        SelectAccelerator {
            super_blocks: Vec::new(),
            zeros_per_super_block: 0,
            zeros_per_block: 0,
            large_super_block_size: 0,
            large_block_size: 0,
        }
    }

    /// Get the size of the select accelerator including space on the heap and the select lookup table if used
    pub fn get_size(&self) -> usize {
        #[allow(unused_assignments)]
        let mut table_space = 0;
        #[cfg(feature = "USE_SELECT_TABLE")] {
            table_space = 2 * size_of::<[[u8; 8]; 256]>();
        }
        size_of::<SelectAccelerator<BIT>>()
        + self.super_blocks.iter().map(SuperBlock::get_size).sum::<usize>()
        + table_space
    }

    /// Initialize the select accelerator using the `bit_vector`
    pub fn init(&mut self, bit_vector: &BitVector) {
        // calculate the parameters as suggested in the lecture
        self.zeros_per_super_block = bit_vector.len().ilog2().pow(2) as usize;
        self.large_super_block_size = self.zeros_per_super_block.pow(2);
        self.large_block_size = bit_vector.len().ilog2() as usize;
        self.zeros_per_block = (self.large_block_size as f64).sqrt() as usize;
        let mut current_super_block_offset = 0;
        let mut next_super_block_offset;

        let mut zeroes = 0;
        for i in 0..bit_vector.len() {
            // loop through the bit vector and count zeros/ones
            zeroes += if BIT { bit_vector.access(i) } else { 1 - bit_vector.access(i) };
            if zeroes != self.zeros_per_super_block && i != bit_vector.len()-1 {
                continue;
            }
            // if we found enough zeroes/ones for a super block or the bit vector ends construct a new super block
            next_super_block_offset = i+1;
            // either create a small or large super block depending on the size of the super block which is the difference between the `current_super_block_offset` and the `next_super_block_offset`
            if next_super_block_offset - current_super_block_offset >= self.large_super_block_size {
                self.super_blocks.push(self.create_large_super_block(bit_vector, current_super_block_offset..next_super_block_offset));
            } else {
                self.super_blocks.push(self.create_small_super_block(bit_vector, current_super_block_offset..next_super_block_offset));
            }
            zeroes = 0;
            current_super_block_offset = next_super_block_offset;
        }

        // Testing has shown that it is faster to shrink the `super_blocks` vector than to loop through the bit vector and calculating the number of super blocks in advance.
        self.super_blocks.shrink_to_fit();
    }

    /// Creates a lookup table for the bits inside `range` inside `bit_vector`
    /// the i-th entry in the vec holds the global indices in the `bit_vector` to the i-th zero/one inside the `range`
    fn calc_select_table(bit_vector: &BitVector, range: Range<usize>) -> Vec<usize> {
        let mut select_table = Vec::new();
        for j in range {
            if bit_vector.access(j) == if BIT { 1 } else { 0 } {
                // store global index directly, so we don't have to calculate it later
                // we also don't have to store super block offsets anymore
                select_table.push(j);
            }
        }
        // shrink to the actual required size
        select_table.shrink_to_fit();
        select_table
    }

    /// Creates a large super block for the provided `super_block_range`
    #[inline]
    fn create_large_super_block(&self, bit_vector: &BitVector, super_block_range: Range<usize>) -> SuperBlock<BIT> {
        LargeSuperBlock{ select_table: Self::calc_select_table(bit_vector, super_block_range) }
    }

    /// Creates a small super block of the provided `super_block_range`
    #[inline]
    fn create_small_super_block(&self, bit_vector: &BitVector, super_block_range: Range<usize>) -> SuperBlock<BIT> {
        let mut blocks = Vec::new();
        let mut current_block_offset = super_block_range.start;
        let mut next_block_offset;

        let mut zeroes = 0;
        for j in super_block_range.clone() {
            // loop through the bit vector and count zeros/ones
            zeroes += if BIT { bit_vector.access(j) } else { 1 - bit_vector.access(j) };
            if zeroes != self.zeros_per_block && j != super_block_range.end-1 {
                continue;
            }
            // if we found enough zeroes/ones for a block or the super block ends construct a new block
            next_block_offset = j+1;
            // either create a small or large block depending on the size of the block which is the difference between the `current_block_offset` and the `next_block_offset`
            if next_block_offset - current_block_offset >= self.large_block_size {
                blocks.push(LargeBlock { select_table:  Box::new(Self::calc_select_table(bit_vector, current_block_offset..next_block_offset)) });
            } else {
                blocks.push(SmallBlock { offset: current_block_offset });
            }
            zeroes = 0;
            current_block_offset = next_block_offset;
        }

        // shrink to the actual required size
        blocks.shrink_to_fit();
        SmallSuperBlock { blocks }
    }

    /// Get the position of the `index`-th zero/one inside the `bit_vector`
    #[inline]
    pub fn select(&self, index: usize, bit_vector: &BitVector) -> usize {
        let super_block_index = index / self.zeros_per_super_block;
        match &self.super_blocks[super_block_index] {
            // If the super block is large simply return the lookup table result.
            // We have to adjust the index, so it requests the i-th zero/one inside the current super block.
            // Because we store the global index inside the lookup table we don't have to do anymore calculations or store the index of the super block itself.
            LargeSuperBlock{ select_table} => select_table[index % self.zeros_per_super_block],
            // If the super block is small calculate the block index and look inside it
            SmallSuperBlock{ blocks } => {
                let block_index = (index % self.zeros_per_super_block) / self.zeros_per_block;
                match &blocks[block_index] {
                    // If the block is large simply return the lookup table result.
                    // We have to adjust the index, so it requests the i-th zero/one inside the current block.
                    // Because we store the global index inside the lookup table we don't have to do anymore calculations or store the index of the block itself.
                    LargeBlock{ select_table} => select_table[(index % self.zeros_per_super_block) % self.zeros_per_block],
                    // If the block is small first get the block from the bit_vector using the `offset`.
                    // After that we get the index using the select lookup table.
                    // We have to adjust the index, so it requests the i-th zero/one inside the current block.
                    // We also have to add the offset of the block inside the bit vector because select_with_table will only return a local starting at the start of the block
                    SmallBlock{ offset} => {
                        offset
                            + select_with_table(BIT, bit_vector.access_block(*offset), (index % self.zeros_per_super_block) % self.zeros_per_block).expect("No ith zero/one found in block")
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