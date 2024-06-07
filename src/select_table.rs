pub fn select_with_table(bit: bool, data: usize, index: usize) -> Option<usize> {
    //TODO: implement table
    let mut data = data;
    let mut zero_counter = 0;
    for i in 0..64 {
        if data & 1 == if bit { 1 } else { 0 } {
            if zero_counter == index {
                return Some(i);
            }
            zero_counter += 1;
        }
        data >>= 1;
    }
    None
}

#[cfg(test)]
pub mod test {
    use crate::select_table::select_with_table;

    #[test]
    fn test_select_with_table() {
        let input = 0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11110111;
        let mut data = input;
        let mut zero_counter = 0;
        for i in 0..64 {
            println!("{i}");
            if data & 1 == 0 {
                assert_eq!(select_with_table(false, input, zero_counter), Some(i));
                zero_counter += 1;
            }
            data >>= 1;
        }

    }
}
