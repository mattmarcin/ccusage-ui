use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub exit_code: Option<i32>,
    pub retryable: bool,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: None,
            exit_code: None,
            retryable,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_exit_code(mut self, exit_code: Option<i32>) -> Self {
        self.exit_code = exit_code;
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("ccusage was not found")]
    NotInstalled { details: String },
    #[error("ccusage timed out")]
    Timeout,
    #[error("ccusage exited with a non-zero status")]
    NonZeroExit {
        exit_code: Option<i32>,
        stderr: String,
    },
    #[error("ccusage returned invalid JSON")]
    JsonParse { details: String },
    #[error("invalid request")]
    InvalidRequest { details: String },
    #[error("settings error")]
    Settings { details: String },
    #[error("cache error")]
    Cache { details: String },
    #[error("I/O error")]
    Io { details: String },
}

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        match value {
            AppError::NotInstalled { details } => ApiError::new(
                "notInstalled",
                "ccusage is not installed or could not be found.",
                false,
            )
            .with_details(details),
            AppError::Timeout => {
                ApiError::new("timeout", "ccusage took too long to respond.", true)
            }
            AppError::NonZeroExit { exit_code, stderr } => {
                ApiError::new("nonZeroExit", "ccusage exited with an error.", true)
                    .with_exit_code(exit_code)
                    .with_details(stderr)
            }
            AppError::JsonParse { details } => ApiError::new(
                "jsonParse",
                "ccusage returned JSON that this app could not parse.",
                false,
            )
            .with_details(details),
            AppError::InvalidRequest { details } => {
                ApiError::new("invalidRequest", "The usage request is invalid.", false)
                    .with_details(details)
            }
            AppError::Settings { details } => ApiError::new(
                "settings",
                "The settings could not be loaded or saved.",
                false,
            )
            .with_details(details),
            AppError::Cache { details } => ApiError::new(
                "cache",
                "The local cache could not be read or written.",
                true,
            )
            .with_details(details),
            AppError::Io { details } => {
                ApiError::new("io", "A local file or process operation failed.", true)
                    .with_details(details)
            }
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io {
            details: value.to_string(),
        }
    }
}
