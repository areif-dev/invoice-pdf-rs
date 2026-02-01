use std::fmt::{Debug, Display};

pub struct Error {
    kind: ErrorKind,
    context: Vec<String>,
}

pub enum ErrorKind {
    Io(std::io::Error),
    FantocciniNewSession(fantoccini::error::NewSessionError),
    FantocciniCmdError(fantoccini::error::CmdError),
    FantocciniPrintError(fantoccini::error::PrintConfigurationError),
    Other(String),
}

pub trait AddContext<T> {
    fn add_context(self, ctx: &str) -> Result<T, Error>;
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut context = self.context.clone();
        context.reverse();
        let context = if context.is_empty() {
            String::from("no context")
        } else {
            context.join(" -> ")
        };
        write!(f, "{context}")
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error {
            context: vec![format!("{:?}", value)],
            kind: ErrorKind::Io(value),
        }
    }
}

impl From<fantoccini::error::PrintConfigurationError> for Error {
    fn from(value: fantoccini::error::PrintConfigurationError) -> Self {
        Error {
            context: vec![format!("{:?}", value)],
            kind: ErrorKind::FantocciniPrintError(value),
        }
    }
}

impl From<fantoccini::error::NewSessionError> for Error {
    fn from(value: fantoccini::error::NewSessionError) -> Self {
        Error {
            context: vec![format!("{:?}", value)],
            kind: ErrorKind::FantocciniNewSession(value),
        }
    }
}

impl From<fantoccini::error::CmdError> for Error {
    fn from(value: fantoccini::error::CmdError) -> Self {
        Error {
            context: vec![format!("{:?}", value)],
            kind: ErrorKind::FantocciniCmdError(value),
        }
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error {
            context: vec![value.to_string()],
            kind: ErrorKind::Other(value),
        }
    }
}

impl Error {
    /// Add more context to the given error. This context will ultimately be displayed to the user
    /// and could be useful for correcting bad input or filing a help ticket.
    ///
    /// Generally a single layer of context should be added for every level that an error is
    /// surfaced. If the error is surfaced all the way to main and not handled there, then all the
    /// context will be displayed to the user in reverse order
    ///
    /// # Arguments
    /// * `context` - Any additional information that would be useful for the user to see if the
    /// error is surfaced to them
    pub fn add_context(self, context: &str) -> Error {
        let mut existing = self.context.clone();
        existing.push(context.to_string());
        Self {
            context: existing,
            ..self
        }
    }
}

impl<T> AddContext<T> for Result<T, Error> {
    fn add_context(self, ctx: &str) -> Result<T, Error> {
        match self {
            Ok(d) => Ok(d),
            Err(e) => Err(e.add_context(ctx)),
        }
    }
}
