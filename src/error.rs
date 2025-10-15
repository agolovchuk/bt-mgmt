pub enum AppError {
    Failed(String),
    NmError(zbus::Error),
    Serialize(serde_json::Error),
}

impl From<AppError> for bluer::gatt::local::ReqError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Failed(message) => {
                println!("Failed: {}", message);
                Self::Failed
            }
            AppError::NmError(_) => Self::Failed,
            AppError::Serialize(_) => Self::Failed,
        }
    }
}
