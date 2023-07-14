use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::Com::CLSCTX_INPROC_SERVER;
use windows::Win32::System::Com::CoCreateInstance;
use windows::Win32::System::Com::CoInitialize;
use windows::Win32::System::Com::IPersistFile;
use windows::Win32::System::Registry::HKEY;
use windows::Win32::System::Registry::HKEY_CURRENT_USER;
use windows::Win32::System::Registry::REG_SZ;
use windows::Win32::System::Registry::RegCloseKey;
use windows::Win32::System::Registry::RegCreateKeyA;
use windows::Win32::System::Registry::RegDeleteKeyA;
use windows::Win32::System::Registry::RegSetValueExA;
use windows::Win32::UI::Shell::IShellLinkA;
use windows::core::BSTR;
use windows::core::ComInterface;
use windows::core::PCSTR;
use windows::core::PCWSTR;
use winreg::RegKey;

use std::path::Path;

use crate::helpers::like::CStringLike;
use crate::workloads::definitions::Product;

use super::GlobalConfig;
use super::GlobalConfigImpl;
use super::error::AppEntryError;
use super::error::CreateSymlinkError;
use super::error::OsError;

pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link_dir: Q, link_name: &str) -> Result<(), CreateSymlinkError> {
    let link = link_dir.as_ref().join(format!("{}.lnk", link_name));

    unsafe {
        _ = CoInitialize(None)?;
    }

    let guid = windows::core::GUID::from("00021401-0000-0000-C000-000000000046");

    let linker: IShellLinkA = unsafe { CoCreateInstance(&guid as _, None, CLSCTX_INPROC_SERVER) }?;

    unsafe { linker.SetPath(PCSTR::from_raw(original.as_ref().to_str().unwrap().as_ptr_nul())) }?;

    let file = linker.cast::<IPersistFile>()?;
    unsafe { file.Save(PCWSTR::from_raw(BSTR::from(link.to_str().unwrap()).into_raw()), true) }?;

    Ok(())  
}

pub fn break_symlink_file<P: AsRef<Path>>(link_dir: P, link_name: &str) -> std::io::Result<()> {
    std::fs::remove_file(link_dir.as_ref().join(format!("{}.lnk", link_name)))
}

pub fn create_app_entry(app: &Product) -> Result<(), AppEntryError> {
    let maintinance_tool_path = Path::join(Path::new(&app.target_directory), "maintinance.exe");

    unsafe {
        let mut hkey = HKEY::default();  
        OsError::into_result(RegCreateKeyA(HKEY_CURRENT_USER, PCSTR::from_raw(format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", app.name).as_ptr_nul()),&mut hkey as *mut _))?;
        OsError::into_result(RegSetValueExA(hkey, PCSTR::from_raw("DisplayName".as_ptr_nul()), 0, REG_SZ, Some(app.name.as_bytes())))?;
        OsError::into_result(RegSetValueExA(hkey, PCSTR::from_raw("InstallLocation".as_ptr_nul()), 0, REG_SZ,Some(app.target_directory.as_bytes())))?;
        OsError::into_result(RegSetValueExA(hkey, PCSTR::from_raw("UninstallString".as_ptr_nul()), 0, REG_SZ, Some(format!(r#"{} /uninstall"#, maintinance_tool_path.to_str().unwrap()).as_bytes())))?;
        OsError::into_result(RegCloseKey(hkey))?;
    }

    Ok(())
}

pub fn delete_app_entry(app: &Product) -> Result<(), AppEntryError> {
    unsafe {
        let r = OsError::into_result(RegDeleteKeyA(HKEY_CURRENT_USER, PCSTR::from_raw(format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", app.name).as_ptr_nul())))?;
        Ok(r)
    }    
}

impl GlobalConfigImpl for GlobalConfig {
    fn new() -> Self {
        Self {  }
    }

    fn set(&self, key: String, name: String, value: String) -> Result<(), OsError> {
        let mut hkey = HKEY::default();
        unsafe {
            OsError::into_result(RegCreateKeyA(HKEY_CURRENT_USER, PCSTR::from_raw(key.as_ptr_nul()),&mut hkey as *mut _))?;
            OsError::into_result(RegSetValueExA(hkey, PCSTR::from_raw(name.as_ptr_nul()), 0, REG_SZ, Some(value.as_bytes())))?;
        }
        Ok(())
    }

    fn get(&self, key: String, name: String) -> Result<String, OsError> {
        let hklm = RegKey::predef(HKEY_CURRENT_USER.0);
        let cur_ver = hklm.open_subkey(key)
            .map_err(|err| OsError::Other(err.to_string()))?;
        let v: String = cur_ver.get_value(name)
            .map_err(|err| OsError::Other(err.to_string()))?;

        Ok(v)
    }

    fn delete(&self, key: String) -> Result<(), OsError> {
        let hklm = RegKey::predef(HKEY_CURRENT_USER.0);
        hklm.delete_subkey_all(key)
            .map_err(|err| OsError::Other(err.to_string()))?;
        Ok(())
    }
}

//////////
/// Errors
/// 
/// //////
impl std::convert::From<windows::core::Error> for CreateSymlinkError {
    fn from(value: windows::core::Error) -> Self {
        CreateSymlinkError::OsError(value.into())
    }
}

impl std::convert::From<windows::core::Error> for AppEntryError {
    fn from(value: windows::core::Error) -> Self {
        AppEntryError::OsError(value.into())
    }
}

impl std::convert::From<windows::core::Error> for OsError {
    fn from(value: windows::core::Error) -> Self {
        OsError::Other(value.to_string())
    }
}

impl OsError {
    pub fn into_result(value: WIN32_ERROR) -> Result<(), OsError> {
        if value.0 == 0 {
            return Ok(());
        }

        let err = windows::core::Error::from(value.to_hresult());
        Err(OsError::Other(err.to_string()))
    } 
}

//////////
/// Tests
/// 
/// //////
#[cfg(test)]
mod tests {
    use crate::sys::break_symlink_file;

    #[test]
    fn test_symlink() {
        use super::symlink_file;
        use std::path::Path;
        use std::fs::File;
        use std::io::Write;

        let original = Path::new("C:\\Users\\Public\\Desktop\\test.txt");
        let link_dir = Path::new("C:\\Users\\Public\\Desktop");
        let link_name = "test";

        let mut file = File::create(original).unwrap();
        file.write_all(b"Hello, world!").unwrap();

        symlink_file(original, link_dir, link_name).unwrap();
        break_symlink_file(link_dir, link_name).unwrap();

        std::fs::remove_file(original).unwrap();
    }

    #[test]
    fn test_global_config() {
        use super::GlobalConfigImpl;
        use super::GlobalConfig;

        let config = GlobalConfig::new();
        let key = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\test".to_string();
        config.set(key.clone(), "DisplayName".to_string(), "test".to_string()).unwrap();
        let v = config.get(key.clone(), "DisplayName".to_string()).unwrap();
        config.delete(key).unwrap();
        assert_eq!(v, "test");
    }
}