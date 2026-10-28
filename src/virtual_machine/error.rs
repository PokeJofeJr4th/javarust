use std::fmt::Display;

impl std::error::Error for Error {}

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

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClassResolution(c) => write!(f, "Failed to resolve class {c:?}"),
            Self::Misc(s) => write!(f, "{s}"),
            Self::ThreadKill => write!(f, "Thread killed"),
        }
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::Misc(value)
    }
}

pub type Result<T> = core::result::Result<T, Error>;
