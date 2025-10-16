#[derive(Debug)]
pub enum AppError {
    Failed(String),
    ZBusError(zbus::Error),
    NmError(&'static str),
    Serialize(serde_json::Error),
}

impl From<AppError> for bluer::gatt::local::ReqError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Failed(message) => {
                println!("Failed: {}", message);
                Self::Failed
            }
            AppError::ZBusError(_) => Self::Failed,
            AppError::Serialize(_) => Self::Failed,
            AppError::NmError(message) => {
                println!("NmError: {}", message);
                Self::Failed
            }
        }
    }
}
