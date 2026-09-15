use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("window {0:#x} is gone")]
    WindowGone(isize),
    #[error("no monitor matches {0}")]
    NoSuchMonitor(String),
    #[error("cannot read the image at {0}")]
    NoSuchImage(String),
    #[error("the {0} thread is already running")]
    AlreadyRunning(&'static str),
    #[error("{context}: {source}")]
    Win32 {
        context: &'static str,
        source: WinError,
    },
}

/// A Win32 HRESULT plus the call that produced it, kept as plain data so the
/// error type stays `Send + Sync` and can cross the Tauri command boundary.
#[derive(Debug, Clone, Copy)]
pub struct WinError(pub i32);

impl fmt::Display for WinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08X}", self.0)
    }
}

impl std::error::Error for WinError {}

impl Error {
    pub fn win32(context: &'static str, code: i32) -> Self {
        Error::Win32 {
            context,
            source: WinError(code),
        }
    }
}

#[cfg(windows)]
impl From<(&'static str, windows::core::Error)> for Error {
    fn from((context, err): (&'static str, windows::core::Error)) -> Self {
        Error::win32(context, err.code().0)
    }
}
