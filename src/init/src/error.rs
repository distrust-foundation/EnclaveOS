use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub struct Report {
    message: String,
}

impl Report {
    pub fn new(message: impl std::fmt::Display) -> Self {
        Report {
            message: message.to_string()
        }
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl<E> From<E> for Report
where
    E: Error + Send + Sync + 'static,
{
    fn from(value: E) -> Self {
        Report {
            message: value.to_string(),
        }
    }
}

pub trait Context<T, E> {
    fn context<C>(self, context: C) -> Result<T, Report>
    where
        C: Display + Send + Sync + 'static;

    fn with_context<C, F>(self, context_fn: F) -> Result<T, Report>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T, E> Context<T, E> for Result<T, E>
where
    E: Error + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> Result<T, Report>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|e| Report {
            message: format!("{context}: {e}"),
        })
    }

    fn with_context<C, F>(self, context_fn: F) -> Result<T, Report>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| Report {
            message: format!("{}: {}", context_fn(), e),
        })
    }
}

impl<T> Context<T, Report> for Result<T, Report> {
    fn context<C>(self, context: C) -> Result<T, Report>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|e| Report {
            message: format!("{context}: {e}"),
        })
    }

    fn with_context<C, F>(self, context_fn: F) -> Result<T, Report>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| Report {
            message: format!("{}: {}", context_fn(), e),
        })
    }
}

pub type Result<T, Error = Report> = std::result::Result<T, Error>;

#[macro_export]
macro_rules! error_ {
    ($($arg:tt)*) => { $crate::error::Report::new(format!($($arg)*)) };
}

pub use error_ as error;
