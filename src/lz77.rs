#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    Literal(u8),
    EndOfBlock,
    BackReference { length: u16, distance: u16 },
}
