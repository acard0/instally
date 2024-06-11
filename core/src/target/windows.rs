use chrono::Datelike;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::Com::CLSCTX_INPROC_SERVER;
use windows::Win32::System::Com::CoCreateInstance;
use windows::Win32::System::Com::CoInitialize;
use windows::Win32::System::Com::IPersistFile;
use windows::Win32::System::Registry::HKEY_CURRENT_USER;
use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
use windows::Win32::System::Registry::RegDeleteKeyA;
use windows::Win32::UI::Shell::IShellLinkA;
use windows::core::BSTR;
use windows::core::ComInterface;
use windows::core::PCSTR;
use windows::core::PCWSTR;
use winreg::RegKey;

use std::path::Path;

use crate::helpers;
use crate::helpers::file;
use crate::helpers::file::IoError;
use crate::helpers::like::CStringLike;
use crate::definitions::app::InstallyApp;

use super::GlobalConfig;
use super::GlobalConfigImpl;
use super::error::AppEntryError;
use super::error::SymlinkError;
use super::error::OsError;

pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link_dir: Q, link_name: &str) -> Result<(), SymlinkError> {
    let link = link_dir.as_ref().join(format!("{}.lnk", link_name));

    unsafe {
        _ = CoInitialize(None)?;

        let guid = windows::core::GUID::from("00021401-0000-0000-C000-000000000046");
        let linker: IShellLinkA = CoCreateInstance(&guid as _, None, CLSCTX_INPROC_SERVER)?;
    
        linker.SetPath(PCSTR::from_raw(original.as_ref().to_str().unwrap().as_ptr_nul()))?;
        linker.SetWorkingDirectory(PCSTR::from_raw(original.as_ref().parent().unwrap().to_str().unwrap().as_ptr_nul()))?;
    
        let file = linker.cast::<IPersistFile>()?;
        file.Save(PCWSTR::from_raw(BSTR::from(link.to_str().unwrap()).into_raw()), true)?;
    }

    Ok(())  
}

pub fn break_symlink_file<P: AsRef<Path>>(link_dir: P, link_name: &str) -> Result<(), SymlinkError>  {
    helpers::file::delete(link_dir.as_ref().join(format!("{}.lnk", link_name)))?;
    Ok(())
}

pub fn create_app_entry(app: &InstallyApp, maintenance_tool_name: &str) -> Result<(), AppEntryError> {
    let path = Path::new("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\").join(&app.get_product().name);
    log::info!("Creating app entry at path {:?}", path);
    
    let target_dir = app.get_product().get_relative_target_directory();
    let maintenance_tool_path = target_dir.join(format!("{}.exe", maintenance_tool_name));
    let date = chrono::Local::now();
    let formatted_date = format!("{:02}.{:02}.{:02}", date.year() % 100, date.month(), date.day());

    let hkey = RegKey::predef(HKEY_CURRENT_USER.0);
    let (key, disp) = hkey.create_subkey(path).unwrap();
    key.set_value("DisplayName", &app.get_product().name)?;
    key.set_value("Comments", &app.get_product().name)?;
    key.set_value("EstimatedSize", &((app.get_repository().size / 1000) as u32))?;
    key.set_value("DisplayVersion", &formatted_date)?;
    key.set_value("DisplayIcon", &maintenance_tool_path.to_str().unwrap())?;
    key.set_value("Publisher", &app.get_product().publisher)?;
    key.set_value("URLInfoAbout", &app.get_product().product_url)?;
    key.set_value("HelpLink", &app.get_product().product_url)?;
    key.set_value("URLUpdateInfo", &app.get_product().product_url)?;
    key.set_value("InstallLocation", &target_dir.to_str().unwrap().to_owned())?;
    key.set_value("UninstallString", &format!(r#"{} /uninstall"#, maintenance_tool_path.to_str().unwrap()))?;
    
    Ok(())
}

pub fn delete_app_entry(app: &InstallyApp) -> Result<(), AppEntryError> {
    unsafe {
        let path = format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", app.get_product().name);
        log::info!("Removing app entry at path {}", path);
        
        Ok(OsError::into_result(RegDeleteKeyA(HKEY_CURRENT_USER, PCSTR::from_raw(path.as_ptr_nul())))?)
    }    
}

pub fn create_maintenance_tool(app: &InstallyApp, maintenance_tool_name: &str) -> Result<(), IoError> {
    let exec_path = std::env::current_exe().unwrap();
    let copy_path = std::path::Path::new(&app.get_product().get_relative_target_directory()).join(format!("{}.exe", maintenance_tool_name));
    _ = file::copy_file(exec_path, copy_path)?;
    Ok(())
}

pub fn delete_maintenance_tool(app: &InstallyApp, maintenance_tool_name: &str) -> Result<(), IoError> {
    let path = std::path::Path::new(&app.get_product().get_relative_target_directory()).join(format!("{}.exe", maintenance_tool_name));
    _ = file::delete(path)?;
    Ok(())
}

impl GlobalConfigImpl for GlobalConfig {
    fn new() -> Self {
        Self {  }
    }

    fn set(&self, key: String, name: String, value: String) -> Result<(), OsError> {
        let hklm_str = key.split('\\').collect::<Vec<&str>>();
        let hkey = match hklm_str[0] {
            "HKEY_CURRENT_USER" => RegKey::predef(HKEY_CURRENT_USER.0),
            "HKEY_LOCAL_MACHINE" => RegKey::predef(HKEY_LOCAL_MACHINE.0),
            _ => return Err(OsError::Other(format!("Unsupported HKEY: {}", hklm_str[0])))
        };

        let (key, disp) = hkey.create_subkey(&hklm_str[1..].join("\\")).unwrap();

        key.set_value(name, &value)
            .map_err(|err| OsError::Other(err.to_string()))
    }

    fn get(&self, key: String, name: String) -> Result<String, OsError> {
        let hklm_str = key.split('\\').collect::<Vec<&str>>();
        let hkey = match hklm_str[0] {
            "HKEY_CURRENT_USER" => RegKey::predef(HKEY_CURRENT_USER.0),
            "HKEY_LOCAL_MACHINE" => RegKey::predef(HKEY_LOCAL_MACHINE.0),
            _ => return Err(OsError::Other(format!("Unsupported HKEY: {}", hklm_str[0])))
        };

        let cur_ver = hkey.open_subkey(&hklm_str[1..].join("\\"))
            .map_err(|err| OsError::Other(err.to_string()))?;
        cur_ver.get_value::<String, _>(name)
            .map_err(|err| OsError::Other(err.to_string()))
    }

    fn delete(&self, key: String) -> Result<(), OsError> {
        let hklm_str = key.split('\\').collect::<Vec<&str>>();
        let hkey = match hklm_str[0] {
            "HKEY_CURRENT_USER" => RegKey::predef(HKEY_CURRENT_USER.0),
            "HKEY_LOCAL_MACHINE" => RegKey::predef(HKEY_LOCAL_MACHINE.0),
            _ => return Err(OsError::Other(format!("Unsupported HKEY: {}", hklm_str[0])))
        };

        hkey.delete_subkey_all(&hklm_str[1..].join("\\"))
            .map_err(|err| OsError::Other(err.to_string()))
    }
}

//////////
/// Errors
/// 
/// //////
impl std::convert::From<windows::core::Error> for SymlinkError {
    fn from(value: windows::core::Error) -> Self {
        SymlinkError::Os(value.into())
    }
}

impl std::convert::From<windows::core::Error> for AppEntryError {
    fn from(value: windows::core::Error) -> Self {
        AppEntryError::Os(value.into())
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
        let key = "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\test".to_string();
        config.set(key.clone(), "DisplayName".to_string(), "test".to_string()).unwrap();
        let v = config.get(key.clone(), "DisplayName".to_string()).unwrap();
        config.delete(key).unwrap();
        assert_eq!(v, "test");
    }
}