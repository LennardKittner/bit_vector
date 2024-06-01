use std::{cmp, f64};
use std::mem::size_of;

type Unit = u64;
const UNIT_SIZE_BITS: usize = size_of::<Unit>()*8;
const UNIT_SIZE_BYTES: usize = size_of::<Unit>();

const MASK_LOOKUP: [u32; 32] = [
    0b00000000000000000000000000000000,
    0b00000000000000000000000000000001,
    0b00000000000000000000000000000011,
    0b00000000000000000000000000000111,
    0b00000000000000000000000000001111,
    0b00000000000000000000000000011111,
    0b00000000000000000000000000111111,
    0b00000000000000000000000001111111,
    0b00000000000000000000000011111111,
    0b00000000000000000000000111111111,
    0b00000000000000000000001111111111,
    0b00000000000000000000011111111111,
    0b00000000000000000000111111111111,
    0b00000000000000000001111111111111,
    0b00000000000000000011111111111111,
    0b00000000000000000111111111111111,
    0b00000000000000001111111111111111,
    0b00000000000000011111111111111111,
    0b00000000000000111111111111111111,
    0b00000000000001111111111111111111,
    0b00000000000011111111111111111111,
    0b00000000000111111111111111111111,
    0b00000000001111111111111111111111,
    0b00000000011111111111111111111111,
    0b00000000111111111111111111111111,
    0b00000001111111111111111111111111,
    0b00000011111111111111111111111111,
    0b00000111111111111111111111111111,
    0b00001111111111111111111111111111,
    0b00011111111111111111111111111111,
    0b00111111111111111111111111111111,
    0b01111111111111111111111111111111,
];

pub struct BitVector {
    data: Vec<Unit>,
    // holds blocks worst case need 10bit per entry
    blocks: Vec<u16>,
    // holds super blocks worst case need 64bit per entry
    super_blocks: Vec<usize>,
    len: usize,
    block_size: usize,
    super_block_size: usize,
}

impl BitVector {
    pub fn new() -> Self {
        BitVector {
            data: Vec::new(),
            blocks: Vec::new(),
            super_blocks: Vec::new(),
            len: 0,
            block_size: 0,
            super_block_size: 0,
        }
    }

    // create a BitVector without initializing any helper data structures
    pub fn load_from_string(data: &str) -> Self {
        let data_it :Vec<bool> =data.chars().map(|c| {
            c == '1'
        }).collect();
        let mut bit_vector = Self::new();

        let mut tmp = 0;
        for (i, &b) in data_it.iter().enumerate() {
            if i != 0 && i % UNIT_SIZE_BITS == 0 {
                bit_vector.data.push(tmp);
                tmp = 0;
            }
            if b {
                tmp |= 1 << (i % UNIT_SIZE_BITS);
            }
        }
        bit_vector.data.push(tmp);
        bit_vector.len = data.len();
        bit_vector
    }

    // initializes helper data structures
    pub fn init(&mut self) {
        //TODO: maybe size optimization
        //TODO: maybe avoid access
        //TODO: out of bounds access
        self.block_size = cmp::max(((self.len as f64).log2() / 2f64) as usize, 1);
        self.super_block_size = self.block_size.pow(2);

        // generate super blocks
        self.super_blocks.reserve_exact(self.len / self.super_block_size);
        let mut block_0 = 0;
        for current_bit in 0..self.super_block_size {
            block_0 += self.access(current_bit);
        }
        self.super_blocks.push(block_0);

        for current_super_block in 1..(self.len / self.super_block_size) {
            let mut block = self.super_blocks[current_super_block -1];
            for current_bit in (current_super_block * self.super_block_size)..((current_super_block +1) * self.super_block_size) {
                block += self.access(current_bit);
            }
            self.super_blocks.push(block);
        }

        // generate blocks
        self.blocks.reserve_exact(self.len / self.block_size);
        for current_super_block in 0..(self.len / self.super_block_size) {
            let mut block_0 = 0;
            for i in (current_super_block*self.super_block_size)..(current_super_block*self.super_block_size + self.block_size) {
                block_0 += self.access(i);
            }
            self.blocks.push(block_0 as u16);

            for current_block in 1..self.block_size {
                let mut block = self.blocks[self.block_size * current_super_block + current_block -1] as usize;
                for current_bit in (current_super_block * self.super_block_size + current_block * self.block_size)..(current_super_block * self.super_block_size + (current_block + 1) * self.block_size) {
                    block += self.access(current_bit);
                }
                self.blocks.push(block as u16);
            }
        }
    }

    // get bit at index
    #[inline]
    pub fn access(&self, index: usize) -> usize {
        let vec_index = index / UNIT_SIZE_BITS;
        let unit_index = index % UNIT_SIZE_BITS;

        ((self.data[vec_index] >> unit_index) & 1) as usize
    }

    #[inline]
    fn get_ones(block: u32, index: usize) -> usize {
        let block = block & MASK_LOOKUP[index];
        block.count_ones() as usize
    }

    #[inline]
    fn get_block(&self, index: usize) -> u32 {
        //TODO: avoid access
        let pos = (index / self.block_size) * self.block_size;

        let mut result = 0;
        for i in pos..(pos + self.block_size) {
            result |= self.access(i) << (i - pos)
        }
        result as u32
    }


    // get number of 0/1 before index
    #[inline]
    pub fn rank(&self, bit: bool, index: usize) -> usize {
        let super_block = index / self.super_block_size;
        let block = index / self.block_size;

        let result1 = self.super_blocks.get(super_block.wrapping_sub(1)).unwrap_or(&0);
        let result2 = if block % (self.super_block_size / self.block_size) == 0 { 0 } else { self.blocks[block - 1] as usize };
        let result3 = Self::get_ones(self.get_block(index), index % self.block_size);
        let result = result1 + result2 + result3;

        if bit { result } else { index - result }
    }

    // get position of index-th 0/1
    #[inline]
    pub fn select(&self, bit: bool, index: usize) -> usize {
        todo!("select")
    }
}

#[cfg(test)]
pub mod test {
    use crate::BitVector;
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_load_from_string_and_access() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101010111111000001111010101010100001110101010111101010110011110010011";
        let bit_vector = BitVector::load_from_string(data);

        for (i, c) in data.chars().enumerate() {
            assert_eq!(c == '1', bit_vector.access(i) == 1)
        }
    }

    fn test_init(data: &str) {
        let mut bit_vector = BitVector::load_from_string(data);
        bit_vector.init();
        for (i, super_block) in bit_vector.super_blocks.iter().enumerate() {
            let mut sum = 0;
            for current_bit in 0..((i+1) * bit_vector.super_block_size) {
                sum += bit_vector.access(current_bit);
            }
            assert_eq!(&sum, super_block);
        }

        for (i, block) in bit_vector.blocks.iter().enumerate() {
            let current_super_block = i / bit_vector.block_size;
            let current_block = i % bit_vector.block_size;
            let mut sum = 0;
            for current_bit in (current_super_block * bit_vector.super_block_size)..(current_super_block * bit_vector.super_block_size + (current_block+1) * bit_vector.block_size) {
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

    #[test]
    fn test_rank() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101110000111011100110110101110101111";
        let mut bit_vector = BitVector::load_from_string(data);
        bit_vector.init();
        let mut sum = 0;
        for i in 0..data.len() {
            assert_eq!(bit_vector.rank(true, i), sum);
            assert_eq!(bit_vector.rank(false, i), i - sum);
            sum += bit_vector.access(i);
        }
    }
}