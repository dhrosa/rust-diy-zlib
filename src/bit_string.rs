// Construct a sequence of bytes from a string of 0 and 1 chars. Any other
// characters are ignored. Any incomplete bytes are padded from the right with
// zeroes. Bits within a byte are read in MSB-order.
pub fn bit_string(s: &str) -> Vec<u8> {
    let is_bit = |c: &char| *c == '0' || *c == '1';
    // Bits represented as a sequence of 0s and 1s.
    let bits = s
        .chars()
        .filter(is_bit)
        .map(|c| (c == '1') as u8)
        .collect::<Vec<u8>>();
    let bits = bits.as_slice();
    let mut bytes = Vec::new();
    for chunk in bits.chunks(8) {
        // Transfer bits to our byte starting from MSB.
        let mut byte = 0;
        let mut bit_index = 7;
        for bit in chunk {
            byte |= bit << bit_index;
            bit_index -= 1;
        }
        bytes.push(byte);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_string() {
        assert_eq!(bit_string("0000 0010 1111"), vec![0b10, 0b1111_0000]);
    }
}
