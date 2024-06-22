use std::cmp;
use std::cmp::min;
use std::mem::size_of;
use crate::BitVector;

/// An accelerator used to for rank operations
pub struct RankAccelerator {
    /// Holds blocks.
    /// Each entry contains the number of ones from the beginning of the super block to the end of the block
    /// Worst case needs 10bit per entry because super_block_size <= (floor(log(2⁶⁴) / 2))² = 1024
    blocks: Vec<u16>,
    /// Holds super blocks.
    /// Each entry contains the number of ones from the start of the vector to the end of the super block
    /// Worst case needs 64bit per entry because the vector can store 2⁶⁴ bits
    super_blocks: Vec<usize>,
    /// The size of a block in bits
    block_size: usize,
    /// The size of a super block in bits
    super_block_size: usize,
}

impl RankAccelerator {

    /// Creates an uninitialized rank accelerator
    pub fn new() -> Self {
        RankAccelerator {
            blocks: Vec::new(),
            super_blocks: Vec::new(),
            block_size: 0,
            super_block_size: 0,
        }
    }

    /// Get the size of the rank accelerator including space on the heap
    pub fn get_size(&self) -> usize {
        size_of::<RankAccelerator>()
        + self.blocks.capacity() * size_of::<u16>()
        + self.super_blocks.capacity() * size_of::<usize>()
    }

    /// Initialize the rank accelerator using the `bit_vector`
    pub fn init(&mut self, bit_vector: &BitVector) {
        // calculate block size and super blocks size as suggested in the lecture
        self.block_size = cmp::max((bit_vector.len().ilog2() as f64 / 2f64) as usize, 1);
        self.super_block_size = self.block_size.pow(2);

        // generate super blocks
        // the number of super blocks is already known therefore this will save space and time because the vector does not have to grow
        self.super_blocks.reserve_exact(bit_vector.len().div_ceil(self.super_block_size));
        // create the first super block
        let num_ones_until_enf_of_block_0 = bit_vector.count_ones(0..self.super_block_size);
        self.super_blocks.push(num_ones_until_enf_of_block_0);

        // create subsequent super blocks using the previous block and `count_ones` to count the ones in the current block
        for current_super_block in 1..bit_vector.len().div_ceil(self.super_block_size) {
            let mut num_ones_until_end_of_block = self.super_blocks[current_super_block - 1];
            let super_block_start = current_super_block * self.super_block_size;
            let super_block_end = min((current_super_block + 1) * self.super_block_size, bit_vector.len());
            num_ones_until_end_of_block += bit_vector.count_ones(super_block_start..super_block_end);
            self.super_blocks.push(num_ones_until_end_of_block);
        }

        // generate blocks
        // the number of super blocks is already known therefore this will save space and time because the vector does not have to grow
        self.blocks.reserve_exact(bit_vector.len().div_ceil(self.block_size));
        // for each super block generate the blocks
        for current_super_block in 0..bit_vector.len().div_ceil(self.super_block_size) {
            // create the first block
            let num_ones_until_enf_of_block_0 = bit_vector.count_ones((current_super_block * self.super_block_size)..min(current_super_block * self.super_block_size + self.block_size, bit_vector.len()));
            self.blocks.push(num_ones_until_enf_of_block_0 as u16);

            // create subsequent blocks using the previous block and `count_ones` to count the ones in the current block
            for current_block in 1..self.block_size {
                let block_start = current_super_block * self.super_block_size + current_block * self.block_size;
                let block_end = min(current_super_block * self.super_block_size + (current_block + 1) * self.block_size, bit_vector.len());
                if block_start >= bit_vector.len() {
                    // happens only if the last super block is smaller than super_block_size
                    break;
                }
                let mut block = self.blocks[self.block_size * current_super_block + current_block - 1] as usize;
                block += bit_vector.count_ones(block_start..block_end);
                self.blocks.push(block as u16);
            }
        }
    }

    /// Count the ones until `index` in the `block`
    #[inline]
    fn get_ones(block: u32, index: usize) -> usize {
        // we don't need a lookup table
        // the mask will set all bits after the index to 0
        let mask = (1 << index) - 1;
        let block = block & mask;
        // this will than count the ones only right from the index-th bit
        // There are two advantages to this.
        // First, there is an assembly instruction to count the zeroes/ones inside a word so `count_ones` should be very fast, probably even faster than a memory access to a lookup table.
        // Second, we save a lot of space because a block has at most `block_size` <= `(bit_vector.len().ilog2() as f64 / 2f64)` <= 32 bits. Thus, a lookup table has to have 2³² entries, which is quite large.
        block.count_ones() as usize
    }

    /// Get the number of zero/one's before `index` from the `bit_vector`
    pub fn rank(&self, bit: bool, index: usize, bit_vector: &BitVector) -> usize {
        // calculate super block index
        let super_block = index / self.super_block_size;
        // calculate block index
        let block = index / self.block_size;
        // calculate the start index of the block
        let block_start = (index / self.block_size) * self.block_size;
        
        // Count the ones until the start of the super block
        let result1 = self.super_blocks.get(super_block.wrapping_sub(1)).unwrap_or(&0);
        // Count the ones until the start of the block
        let result2 = if block % (self.super_block_size / self.block_size) == 0 { 0 } else { self.blocks[block - 1] as usize };
        // Count the ones inside the block until index
        let result3 = Self::get_ones(bit_vector.access_block(block_start) as u32, index % self.block_size);

        let result = result1 + result2 + result3;

        // in the range from zero to index we found result many ones => index - result is the number of zeroes in this range
        if bit { result } else { index - result }
    }
}

#[cfg(test)]
pub mod test {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use crate::BitVector;

    fn test_init(data: &str) {
        let mut bit_vector = BitVector::load_from_string(data);
        bit_vector.init_rank_structures();
        let rank_accelerator = bit_vector.rank_accelerator.as_ref().unwrap();
        for (i, super_block) in rank_accelerator.super_blocks.iter().enumerate() {
            let mut sum = 0;
            for current_bit in 0..((i+1) * rank_accelerator.super_block_size) {
                if current_bit >= bit_vector.len() {
                    break;
                }
                sum += bit_vector.access(current_bit);
            }
            assert_eq!(&sum, super_block);
        }

        for (i, block) in rank_accelerator.blocks.iter().enumerate() {
            let current_super_block = i / rank_accelerator.block_size;
            let current_block = i % rank_accelerator.block_size;
            let mut sum = 0;
            for current_bit in (current_super_block * rank_accelerator.super_block_size)..(current_super_block * rank_accelerator.super_block_size + (current_block+1) * rank_accelerator.block_size) {
                if current_bit >= bit_vector.len() {
                    break;
                }
                sum += bit_vector.access(current_bit);
            }
            assert_eq!(sum, *block as usize, "current_super_block: {current_super_block}, current_block: {current_block}");
        }
    }

    #[test]
    fn test_init_power_of_two_random() {
        let data = "01001000101010000111101010101111100100001011100011100011010101001101010010101011111000011010110101010111110101010111000011101110";
        test_init(data);
    }

    #[test]
    fn test_init_not_power_of_two_random() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101110000111011100110110101110101111";
        test_init(data);
    }

    #[test]
    fn test_init_random_large() {
        let mut data = String::new();
        let mut rng = ChaCha8Rng::seed_from_u64(1234567);
        for _i in 0..4096 {
            if rng.gen_range(0..=1) == 1 {
                data += "1";
            } else {
                data += "0";
            }
            test_init(&data);
        }
    }
}