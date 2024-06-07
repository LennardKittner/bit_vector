use std::mem::size_of;
use crate::rank::RankAccelerator;
use crate::select::SelectAccelerator;

mod rank;
mod select;
mod select_table;

type Unit = u64;
const UNIT_SIZE_BITS: usize = size_of::<Unit>()*8;
const UNIT_SIZE_BYTES: usize = size_of::<Unit>();

pub struct BitVector {
    data: Vec<Unit>,
    len: usize,

    rank_accelerator: Option<RankAccelerator>,

    select_accelerator_0: Option<SelectAccelerator<false>>,
    select_accelerator_1: Option<SelectAccelerator<true>>
}

impl BitVector {
    pub fn new() -> Self {
        BitVector {
            data: Vec::new(),
            len: 0,
            rank_accelerator: None,
            select_accelerator_0: None,
            select_accelerator_1: None
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

    pub fn init_rank_structures(&mut self) {
        let mut rank_accelerator = RankAccelerator::new();
        rank_accelerator.init(self);
        self.rank_accelerator = Some(rank_accelerator);
    }

    pub fn init_select_structures(&mut self) {
        let mut select_accelerator_0 = SelectAccelerator::new();
        select_accelerator_0.init(self);
        self.select_accelerator_0 = Some(select_accelerator_0);
        let mut select_accelerator_1 = SelectAccelerator::new();
        select_accelerator_1.init(self);
        self.select_accelerator_1 = Some(select_accelerator_1);
    }

    // initializes helper data structures
    pub fn init(&mut self) {
        self.init_rank_structures();
        self.init_select_structures();
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    // get bit at index
    #[inline]
    pub fn access(&self, index: usize) -> usize {
        let vec_index = index / UNIT_SIZE_BITS;
        let unit_index = index % UNIT_SIZE_BITS;

        ((self.data[vec_index] >> unit_index) & 1) as usize
    }

    // get number of 0/1 before index
    #[inline]
    pub fn rank(&self, bit: bool, index: usize) -> usize {
        self.rank_accelerator.as_ref().expect("Rank acceleration structures not initialized!").rank(bit, index, self)
    }

    // get position of index-th 0/1
    #[inline]
    pub fn select(&self, bit: bool, index: usize) -> usize {
        if bit {
            self.select_accelerator_1.as_ref().expect("Select acceleration structrues not initialized!").select(index)
        } else {
            self.select_accelerator_0.as_ref().expect("Select acceleration structrues not initialized!").select(index)
        }
    }
}

#[cfg(test)]
pub mod test {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use crate::BitVector;

    #[test]
    fn test_load_from_string_and_access() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101010111111000001111010101010100001110101010111101010110011110010011";
        let bit_vector = BitVector::load_from_string(data);

        for (i, c) in data.chars().enumerate() {
            assert_eq!(c == '1', bit_vector.access(i) == 1)
        }
    }

    #[test]
    fn test_rank() {
        let data = "010010001010100001111010101011111001000010111000111000110101010011010100101010111110000110101101010101111101010101110000111011100110110101110101111";
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
        for _ in 0..4096 {
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
                assert_eq!(bit_vector.select(false, current_zero), i);
                current_zero += 1;
            }
            if bit_vector.access(i) == 1 {
                assert_eq!(bit_vector.select(true, current_one), i);
                current_one += 1;
            }
        }
    }
}