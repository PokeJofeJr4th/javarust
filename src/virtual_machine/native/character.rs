use std::fmt::Display;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Char(pub u16);

impl Display for Char {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match char::from_u32(self.0 as u32) {
            Some(c) => write!(f, "{c}"),
            None => write!(f, "?"),
        }
    }
}
