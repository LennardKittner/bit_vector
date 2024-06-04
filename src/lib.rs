use std::mem::size_of;
use crate::rank::RankAccelerator;

mod rank;

type Unit = u64;
const UNIT_SIZE_BITS: usize = size_of::<Unit>()*8;
const UNIT_SIZE_BYTES: usize = size_of::<Unit>();

pub struct BitVector {
    data: Vec<Unit>,
    len: usize,

    rank_accelerator: Option<RankAccelerator>,

    // select structures
    select_super_block_offsets: Vec<usize>,
    select_super_blocks: Vec<SelectSuperBlock>,
    select_zeros_per_super_block: usize,
    select_large_super_block_size: usize,
    select_large_block_size: usize
}

enum SelectSuperBlock {
    LargeSuperBlock{
        select_table: Vec<usize>
    },
    SmallSuperBlock{
        blocks: Vec<SelectBlock>
    },
}

enum SelectBlock {
    LargeBlock{
        select_table: Vec<usize>
    },
    SmallBlock
}

impl BitVector {
    pub fn new() -> Self {
        BitVector {
            data: Vec::new(),
            len: 0,
            rank_accelerator: None,
            select_super_block_offsets: Vec::new(),
            select_super_blocks: Vec::new(),
            select_zeros_per_super_block: 0,
            select_large_super_block_size: 0,
            select_large_block_size: 0
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
        // self.select_zeros_per_super_block = self.len.ilog2().pow(2) as usize;
        // self.select_large_super_block_size = self.select_zeros_per_super_block.pow(2);
        // self.select_large_block_size = (self.len.ilog2() as f64).sqrt() as usize;
        // self.select_super_block_offsets.push(0);
        // let mut zeroes = 0;
        // for i in 0..self.len {
        //     zeroes = 1 - self.access(i);
        //     if zeroes == self.select_zeros_per_super_block {
        //         zeroes = 0;
        //         self.select_super_block_offsets.push(i);
        //         let next_block = self.select_super_block_offsets.len();
        //         if self.select_super_block_offsets[next_block] - self.select_super_block_offsets[next_block-1] >= self.select_large_super_block_size {
        //             // large super block
        //             let mut select_table = Vec::new();
        //             for j in self.select_super_block_offsets[next_block-1]..self.select_super_block_offsets[next_block] {
        //                 if self.access(j) == 0 {
        //                     select_table.push(j);
        //                 }
        //             }
        //             self.select_super_blocks.push(LargeSuperBlock{ select_table })
        //         } else {
        //             // small super block
        //             let mut select_block_offsets = Vec::new();
        //             let mut select_blocks = Vec::new();
        //             select_block_offsets.push(0);
        //             for j in self.select_super_block_offsets[next_block-1]..self.select_super_block_offsets[next_block] {
        //                 zeroes = 1 - self.access(j);
        //                if zeroes = self.select_large_block_size {
        //                    zeroes = 0;
        //                    select_block_offsets.push(j);
        //                    let next_block = select_block_offsets.len();
        //                     if select_block_offsets[next_block] - select_block_offsets[next_block-1] >= self.select_large_block_size {
        //                         // large block
        //                         let mut select_table = Vec::new();
        //                         for j in self.select_super_block_offsets[next_block-1]..self.select_super_block_offsets[next_block] {
        //                             if self.access(j) == 0 {
        //                                 select_table.push(j);
        //                             }
        //                         }
        //                         select_blocks.push( LargeBlock{ select_table });
        //                     } else {
        //                         select_blocks.push(SmallBlock);
        //                     }
        //                }
        //             }
        //         }
        //     }
        // }
    }

    // initializes helper data structures
    pub fn init(&mut self) {
        self.init_rank_structures();
        self.init_select_structures();
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
        todo!("select")
    }
}

#[cfg(test)]
pub mod test {
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
        bit_vector.init();
        let mut sum = 0;
        for i in 0..data.len() {
            assert_eq!(bit_vector.rank(true, i), sum);
            assert_eq!(bit_vector.rank(false, i), i - sum);
            sum += bit_vector.access(i);
        }
    }
}