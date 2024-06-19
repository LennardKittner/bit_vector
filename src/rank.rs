use std::cmp;
use std::cmp::min;
use std::mem::size_of;
use crate::BitVector;

pub struct RankAccelerator {
    // holds blocks. Worst case needs 10bit per entry
    blocks: Vec<u16>,
    // holds super blocks. Worst case needs 64bit per entry
    super_blocks: Vec<usize>,
    block_size: usize,
    super_block_size: usize,
}

impl RankAccelerator {
    pub fn new() -> Self {
        RankAccelerator {
            blocks: Vec::new(),
            super_blocks: Vec::new(),
            block_size: 0,
            super_block_size: 0,
        }
    }
    
    pub fn get_size(&self) -> usize {
        size_of::<RankAccelerator>()
        + self.blocks.capacity() * size_of::<u16>()
        + self.super_blocks.capacity() * size_of::<usize>()
    }

    pub fn init(&mut self, bit_vector: &BitVector) {
        self.block_size = cmp::max(((bit_vector.len() as f64).log2() / 2f64) as usize, 1);
        self.super_block_size = self.block_size.pow(2);

        // generate super blocks
        self.super_blocks.reserve_exact(bit_vector.len().div_ceil(self.super_block_size));
        let num_ones_until_enf_of_block_0 = bit_vector.count_ones(0..self.super_block_size);
        self.super_blocks.push(num_ones_until_enf_of_block_0);

        for current_super_block in 1..bit_vector.len().div_ceil(self.super_block_size) {
            let mut num_ones_until_end_of_block = self.super_blocks[current_super_block - 1];
            let super_block_start = current_super_block * self.super_block_size;
            let super_block_end = min((current_super_block + 1) * self.super_block_size, bit_vector.len());
            num_ones_until_end_of_block += bit_vector.count_ones(super_block_start..super_block_end);
            self.super_blocks.push(num_ones_until_end_of_block);
        }

        // generate blocks
        self.blocks.reserve_exact(bit_vector.len().div_ceil(self.block_size));
        for current_super_block in 0..bit_vector.len().div_ceil(self.super_block_size) {
            let num_ones_until_enf_of_block_0 = bit_vector.count_ones((current_super_block * self.super_block_size)..min(current_super_block * self.super_block_size + self.block_size, bit_vector.len()));
            self.blocks.push(num_ones_until_enf_of_block_0 as u16);

            for current_block in 1..self.block_size {
                let block_start = current_super_block * self.super_block_size + current_block * self.block_size;
                let block_end = min(current_super_block * self.super_block_size + (current_block + 1) * self.block_size, bit_vector.len());
                if block_start >= bit_vector.len() {
                    // Happens only if the last super block is smaller than super_block_size
                    break;
                }
                let mut block = self.blocks[self.block_size * current_super_block + current_block - 1] as usize;
                block += bit_vector.count_ones(block_start..block_end);
                self.blocks.push(block as u16);
            }
        }
    }

    #[inline]
    fn get_ones(block: u32, index: usize) -> usize {
        // we don't need a lookup table
        let mask = (1 << index) - 1;
        let block = block & mask;
        block.count_ones() as usize
    }

    pub fn rank(&self, bit: bool, index: usize, bit_vector: &BitVector) -> usize {
        let super_block = index / self.super_block_size;
        let block = index / self.block_size;
        let block_start = (index / self.block_size) * self.block_size;

        let result1 = self.super_blocks.get(super_block.wrapping_sub(1)).unwrap_or(&0);
        let result2 = if block % (self.super_block_size / self.block_size) == 0 { 0 } else { self.blocks[block - 1] as usize };
        let result3 = Self::get_ones(bit_vector.access_block(block_start) as u32, index % self.block_size);

        let result = result1 + result2 + result3;

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