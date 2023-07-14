use self::error::OsError;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

pub mod error;

pub struct GlobalConfig {}
pub trait GlobalConfigImpl {
    fn new() -> Self;
    fn set(&self, key: String, name: String, value: String) -> Result<(), OsError>;
    fn get(&self, key: String, name: String) -> Result<String, OsError>;
    fn delete(&self, key: String) -> Result<(), OsError>;
}