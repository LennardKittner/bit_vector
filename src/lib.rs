use std::mem::size_of;

type Unit = u64;
const UNIT_SIZE_BITS: usize = size_of::<Unit>()*8;
const UNIT_SIZE_BYTES: usize = size_of::<Unit>();

pub struct BitVector {
    data: Vec<Unit>,
    len: usize,
}

impl BitVector {
    pub fn new() -> Self {
        BitVector {
            data: Vec::new(),
            len: 0
        }
    }

    pub fn load_from_string(data: &str) -> Self {
        let data_it :Vec<bool> =data.chars().map(|c| {
            c == '1'
        }).collect();
        let mut bit_vector = Self::new();

        let mut tmp = 0;
        for (i, &b) in data_it.iter().enumerate() {
            if i != 0 && i % UNIT_SIZE_BITS == 0 {
                bit_vector.data.push(tmp);
                println!("{:64b}", tmp);
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

    pub fn init(&self) {
        todo!("init helper structures")
    }

    pub fn access(&self, index: usize) -> bool {
        let vec_index = index / UNIT_SIZE_BITS;
        let unit_index = index % UNIT_SIZE_BITS;

        let a = (self.data[vec_index] >> unit_index) & 1 == 1;
        println!("{a}");
        a
    }

    pub fn rank(&self, bit: bool, index: usize) -> usize {
        todo!("rank")
    }

    pub fn select(&self, bit: bool, index: usize) -> usize {
        todo!("select")
    }
}

#[cfg(test)]
pub mod test {
    use crate::BitVector;

    #[test]
    fn test_load_from_string_and_access() {
        let data = "01001000101010000111101010101111100100001011100011100011010101001101010010101011111000011010110101010111110101010101011111100000111101010101010000111010101011110101";
        let bit_vector = BitVector::load_from_string(data);

        for (i, c) in data.chars().enumerate() {
            println!("{i}");
            assert_eq!(c == '1', bit_vector.access(i))
        }
    }
}