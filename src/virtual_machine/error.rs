#[derive(Debug, Clone)]
pub enum Error {
    ThreadKill,
    ClassResolution(String),
    Misc(String),
}

impl Error {
    pub fn class_resolution(class: &impl ToString) -> Self {
        Self::ClassResolution(class.to_string())
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::Misc(value)
    }
}

pub type Result<T> = core::result::Result<T, Error>;
