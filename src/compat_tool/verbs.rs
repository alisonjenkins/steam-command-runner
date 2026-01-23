use crate::error::AppError;

/// Verbs passed by Steam to compatibility tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// Run the game and wait for it to exit
    WaitForExitAndRun,
    /// Run the game without waiting
    Run,
    /// Get the compatibility data path
    GetCompatPath,
    /// Get the native path
    GetNativePath,
}

impl Verb {
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s.to_lowercase().as_str() {
            "waitforexitandrun" => Ok(Verb::WaitForExitAndRun),
            "run" => Ok(Verb::Run),
            "getcompatpath" => Ok(Verb::GetCompatPath),
            "getnativepath" => Ok(Verb::GetNativePath),
            _ => Err(AppError::UnknownVerb(s.to_string())),
        }
    }

    /// Check if this verb should execute the game
    pub fn should_execute(&self) -> bool {
        matches!(self, Verb::WaitForExitAndRun | Verb::Run)
    }

    /// Check if this verb should wait for the game to exit
    pub fn should_wait(&self) -> bool {
        matches!(self, Verb::WaitForExitAndRun)
    }
}
