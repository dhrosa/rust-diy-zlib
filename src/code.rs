use std::fmt;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Code {
    pub bits: u32,
    pub length: u8,
}

impl Code {
    pub fn append_bit(&self, bit: bool) -> Self {
        Self {
            bits: (self.bits << 1) | (bit as u32),
            length: self.length + 1,
        }
    }
}

// Test-only covenience method for constructing a Code from a string.
impl From<&str> for Code {
    fn from(s: &str) -> Self {
        Code {
            bits: u32::from_str_radix(s, 2).unwrap(),
            length: s.len() as u8,
        }
    }
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.length == 0 {
            return Ok(());
        }
        write!(f, "{:0width$b}", self.bits, width = self.length as usize)
    }
}

impl Default for Code {
    fn default() -> Self {
        Code { bits: 0, length: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_debug() {
        assert_eq!(format!("{:?}", Code { bits: 0, length: 0 }), "");
        assert_eq!(format!("{:?}", Code { bits: 2, length: 3 }), "010");
    }

    #[test]
    fn test_code_from() {
        assert_eq!(Code::from("010"), Code { bits: 2, length: 3 });
    }

    #[test]
    fn test_code_append_bit() {
        assert_eq!(Code::from("010").append_bit(false), Code::from("0100"));
        assert_eq!(Code::from("010").append_bit(true), Code::from("0101"));
    }
}
