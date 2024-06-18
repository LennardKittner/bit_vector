use std::cmp;
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
        //TODO: maybe size optimization
        //TODO: maybe avoid access
        self.block_size = cmp::max(((bit_vector.len() as f64).log2() / 2f64) as usize, 1);
        self.super_block_size = self.block_size.pow(2);

        // generate super blocks
        self.super_blocks.reserve_exact(bit_vector.len() / self.super_block_size);
        let mut block_0 = 0;
        for current_bit in 0..self.super_block_size {
            block_0 += bit_vector.access(current_bit);
        }
        self.super_blocks.push(block_0);

        for current_super_block in 1..(bit_vector.len() / self.super_block_size) {
            let mut block = self.super_blocks[current_super_block - 1];
            for current_bit in (current_super_block * self.super_block_size)..((current_super_block + 1) * self.super_block_size) {
                block += bit_vector.access(current_bit);
            }
            self.super_blocks.push(block);
        }

        // generate blocks
        self.blocks.reserve_exact(bit_vector.len() / self.block_size);
        for current_super_block in 0..(bit_vector.len() / self.super_block_size) {
            let mut block_0 = 0;
            for i in (current_super_block * self.super_block_size)..(current_super_block * self.super_block_size + self.block_size) {
                block_0 += bit_vector.access(i);
            }
            self.blocks.push(block_0 as u16);

            for current_block in 1..self.block_size {
                let mut block = self.blocks[self.block_size * current_super_block + current_block - 1] as usize;
                for current_bit in (current_super_block * self.super_block_size + current_block * self.block_size)..(current_super_block * self.super_block_size + (current_block + 1) * self.block_size) {
                    block += bit_vector.access(current_bit);
                }
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

    #[inline]
    fn get_block(&self, index: usize, bit_vector: &BitVector) -> u32 {
        //TODO: avoid access
        let pos = (index / self.block_size) * self.block_size;

        let mut result = 0;
        for i in pos..(pos + self.block_size) {
            result |= bit_vector.access(i) << (i - pos)
        }
        result as u32
    }

    pub fn rank(&self, bit: bool, index: usize, bit_vector: &BitVector) -> usize {
        let super_block = index / self.super_block_size;
        let block = index / self.block_size;

        let result1 = self.super_blocks.get(super_block.wrapping_sub(1)).unwrap_or(&0);
        let result2 = if block % (self.super_block_size / self.block_size) == 0 { 0 } else { self.blocks[block - 1] as usize };
        let result3 = Self::get_ones(self.get_block(index, bit_vector), index % self.block_size);
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
                sum += bit_vector.access(current_bit);
            }
            assert_eq!(&sum, super_block);
        }

        for (i, block) in rank_accelerator.blocks.iter().enumerate() {
            let current_super_block = i / rank_accelerator.block_size;
            let current_block = i % rank_accelerator.block_size;
            let mut sum = 0;
            for current_bit in (current_super_block * rank_accelerator.super_block_size)..(current_super_block * rank_accelerator.super_block_size + (current_block+1) * rank_accelerator.block_size) {
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