/// Error types with semantic exit codes.
///
/// Every error maps to an exit code (1-4), a machine-readable code, and a
/// recovery suggestion that agents can follow literally.

#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] // Config variant exists to demonstrate exit code 2 pattern
pub enum AppError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Update failed: {0}")]
    Update(String),
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) => 3,
            Self::Config(_) => 2,
            Self::Io(_) => 1,
            Self::Update(_) => 1,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::Config(_) => "config_error",
            Self::Io(_) => "io_error",
            Self::Update(_) => "update_error",
        }
    }

    pub fn suggestion(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "Check arguments with: greeter --help",
            Self::Config(_) => "Check config file at ~/.config/greeter/config.toml",
            Self::Io(_) => "Check file paths and permissions, then retry",
            Self::Update(_) => "Retry later, or install manually via cargo install greeter",
        }
    }
}
