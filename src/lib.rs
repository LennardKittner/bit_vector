use std::mem::size_of;
use std::ops::Range;
use crate::rank::RankAccelerator;
use crate::select::SelectAccelerator;

mod rank;
mod select;
mod select_table;

/// The base type can be changed using features
/// TODO: Select and Rank maybe require larger blocks
#[cfg(feature = "UNIT_U8")]
type Unit = u8;
#[cfg(feature = "UNIT_U16")]
type Unit = u16;
#[cfg(feature = "UNIT_U32")]
type Unit = u32;
#[cfg(feature = "UNIT_U64")]
type Unit = u64;
#[cfg(feature = "UNIT_USIZE")]
type Unit = usize;

const UNIT_SIZE_BITS: usize = Unit::BITS as usize;

/// A bit vector that supports fast rank and select
pub struct BitVector {
    /// The raw bitvector data
    data: Vec<Unit>,
    /// The number of bits in the bit vector
    len: usize,

    /// Used to accelerate rank operations
    rank_accelerator: Option<RankAccelerator>,

    /// Used to accelerate zero select operations
    select_accelerator_0: Option<SelectAccelerator<false>>,
    /// Used to accelerate one select operations
    select_accelerator_1: Option<SelectAccelerator<true>>
}

impl Default for BitVector {
    /// Creates an empty bit vector
    fn default() -> Self {
        Self::new()
    }
}

impl BitVector {
    /// Creates an empty bit vector
    pub fn new() -> Self {
        BitVector {
            data: Vec::new(),
            len: 0,
            rank_accelerator: None,
            select_accelerator_0: None,
            select_accelerator_1: None
        }
    }

    /// Get the size of the bit vector including space on the heap
    pub fn get_size(&self) -> usize {
        self.data.capacity() * size_of::<Unit>() + self.get_size_rank() + self.get_size_select_0() + self.get_size_select_1()
    }

    /// Get the size of the rank accelerator including space on the heap
    pub fn get_size_rank(&self) -> usize {
        if let Some(rank_accelerator) = &self.rank_accelerator {
            rank_accelerator.get_size()
        } else {
            0
        }
    }

    /// Get the size of the select zero accelerator including space on the heap
    pub fn get_size_select_0(&self) -> usize {
        if let Some(select_accelerator_0) = &self.select_accelerator_0 {
            select_accelerator_0.get_size()
        } else {
            0
        }
    }

    /// Get the size of the select one accelerator including space on the heap
    pub fn get_size_select_1(&self) -> usize {
        if let Some(select_accelerator_1) = &self.select_accelerator_1 {
            select_accelerator_1.get_size()
        } else {
            0
        }    
    }

    /// Creates a BitVector without initializing any accelerator structures from `data`
    pub fn load_from_string(data: &str) -> Self {
        let data_it :Vec<bool> =data.chars().map(|c| {
            c == '1'
        }).collect();
        let mut bit_vector = Self::new();

        let mut tmp = 0;
        for (i, &b) in data_it.iter().enumerate() {
            // every UNIT_SIZE_BITS push tmp into the raw data vector
            if i != 0 && i % UNIT_SIZE_BITS == 0 {
                bit_vector.data.push(tmp);
                tmp = 0;
            }
            // if one store into tmp
            if b {
                tmp |= 1 << (i % UNIT_SIZE_BITS);
            }
        }
        bit_vector.data.push(tmp);
        bit_vector.len = data.len();
        // shrink to fit the data
        bit_vector.data.shrink_to_fit();
        bit_vector
    }

    /// Creates the rank accelerator
    pub fn init_rank_structures(&mut self) {
        let mut rank_accelerator = RankAccelerator::new();
        rank_accelerator.init(self);
        self.rank_accelerator = Some(rank_accelerator);
    }

    /// Creates the select accelerators
    pub fn init_select_structures(&mut self) {
        let mut select_accelerator_0 = SelectAccelerator::new();
        select_accelerator_0.init(self);
        self.select_accelerator_0 = Some(select_accelerator_0);
        let mut select_accelerator_1 = SelectAccelerator::new();
        select_accelerator_1.init(self);
        self.select_accelerator_1 = Some(select_accelerator_1);
    }

    /// Initializes accelerators structures
    pub fn init(&mut self) {
        self.init_rank_structures();
        self.init_select_structures();
    }

    /// Get the length of the vector
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the vector is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the bit at `index`
    #[inline]
    pub fn access(&self, index: usize) -> usize {
        // calculate the word index
        let vec_index = index / UNIT_SIZE_BITS;
        // calculate the bit inside the word
        let unit_index = index % UNIT_SIZE_BITS;

        (self.data[vec_index] >> unit_index) & 1
    }

    /// Get the word starting at `index`
    #[inline]
    pub fn access_block(&self, index: usize) -> Unit {
        let vec_index = index / UNIT_SIZE_BITS;
        let shift = index % UNIT_SIZE_BITS;
        // the lower part of the block
        let lower = self.data[vec_index] >> shift;
        if vec_index == self.data.len()-1 || shift == 0 {
            return lower;
        }
        // the upper part of the block
        let upper = self.data[vec_index+1] << (UNIT_SIZE_BITS - shift);
        lower | upper
    }

    /// Get the number of one bits in the `range`
    #[inline]
    pub fn count_ones(&self, range: Range<usize>) -> usize {
        let mut result = 0;
        let blocks: Vec<Unit> = range.clone().step_by(UNIT_SIZE_BITS).map(|i| self.access_block(i)).collect();
        
        // Count all blocks that are fully container in the range efficiently using count_ones
        for block in blocks.iter().take(blocks.len() - 1) {
            result += block.count_ones() as usize;
        }
        // calculate a bit maks to count the ones in the last block which maybe only partial in the range
        let mask = if (range.end - range.start) % UNIT_SIZE_BITS == 0 {
            0
        } else {
            (1 << ((range.end - range.start) % UNIT_SIZE_BITS)) - 1
        };
        let last_block = blocks.last().unwrap();
        let remaining = (last_block & mask).count_ones() as usize;
        result + remaining
    }

    /// Get the number of zero/one's before `index`
    #[inline]
    pub fn rank(&self, bit: bool, index: usize) -> usize {
        self.rank_accelerator.as_ref().expect("Rank acceleration structures not initialized!").rank(bit, index, self)
    }

    /// Get the position of the `index`-th zero/one
    #[inline]
    pub fn select(&self, bit: bool, index: usize) -> usize {
        // index -1 because select_accelerator is zero based
        if bit {
            self.select_accelerator_1.as_ref().expect("Select acceleration structures not initialized!").select(index-1, self)
        } else {
            self.select_accelerator_0.as_ref().expect("Select acceleration structures not initialized!").select(index-1, self)
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::cmp::min;
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use crate::{BitVector, UNIT_SIZE_BITS};

    #[test]
    fn test_load_from_string_and_access() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101010111111000001111010101010100001110101010111101010110011110010011";
        let bit_vector = BitVector::load_from_string(data);

        for (i, c) in data.chars().enumerate() {
            assert_eq!(c == '1', bit_vector.access(i) == 1)
        }
    }

    #[test]
    fn test_rank_small() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101110000111011100110110101110101111";
        test_rank(data);
    }

    #[test]
    fn test_rank_large() {
        let mut data = String::new();
        let mut rng = ChaCha8Rng::seed_from_u64(1234567);
        for _ in 0..524288 {
            if rng.gen_range(0..=1) == 0 {
                data += "0";
            } else {
                data += "1";
            }
        }
        test_rank(&data);
    }

    fn test_rank(data: &str) {
        let mut bit_vector = BitVector::load_from_string(data);
        bit_vector.init_rank_structures();
        let mut sum = 0;
        for i in 0..data.len() {
            assert_eq!(bit_vector.rank(true, i), sum);
            assert_eq!(bit_vector.rank(false, i), i - sum);
            sum += bit_vector.access(i);
        }
    }

    #[test]
    fn test_select_small() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101110000111011100110110101110101111";
        test_select(data);
    }
    
    #[test]
    fn test_select_large() {
        let mut data = String::new();
        let mut rng = ChaCha8Rng::seed_from_u64(1234567);
        for _ in 0..524288 { //524288
            if rng.gen_range(0..=1) == 0 {
                data += "0";
            } else {
                data += "1";
            }
        }
        test_select(&data);
    }
    

    fn test_select(data: &str) {
        let mut bit_vector = BitVector::load_from_string(data);
        bit_vector.init_select_structures();
        let mut current_zero = 0;
        let mut current_one = 0;
        for i in 0..data.len() {
            if bit_vector.access(i) == 0 {
                current_zero += 1;
                assert_eq!(bit_vector.select(false, current_zero), i);
            }
            if bit_vector.access(i) == 1 {
                current_one += 1;
                let result = bit_vector.select(true, current_one);
                assert_eq!(result, i);
            }
        }
    }

    #[test]
    fn test_access_block() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101110000111011100110110101110101111";
        let bit_vector = BitVector::load_from_string(data);
        for i in 0..data.len() {
            let mut data = 0;
            for j in i..min(UNIT_SIZE_BITS + i, bit_vector.len) {
                data |= bit_vector.access_block(j) << (j - i);
            }
            assert_eq!(data, bit_vector.access_block(i));
        }
    }

    #[test]
    fn test_count_ones() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101110000111011100110110101110101111";
        let bit_vector = BitVector::load_from_string(data);
        let start = 0;
        let end = 9;
        let mut zeroes = 0;
        for i in start.. end {
            zeroes += bit_vector.access(i);
        }
        assert_eq!(zeroes, bit_vector.count_ones(start..end));
    }
}