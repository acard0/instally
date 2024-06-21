use std::path::Path;

use helpers::file::IoError;
use serde::Deserialize;
use rust_i18n::error::*;
use convert_case::*;
use crate::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum SerializationError {
    #[error("{}", .0.get_message_key())]
    Io(#[from] IoError),

    #[error("json-error")]
    Engine(#[from] serde_json::error::Error),
}

pub fn to_json<T>(value: &T) -> Result<String, SerializationError>
where T: ?Sized + serde::Serialize 
{
    let json = serde_json::to_string_pretty(value)?;
    Ok(json)
}

pub fn from_json<'de, T>(json: &'de str) -> Result<T, SerializationError>
where T: ?Sized + serde::Deserialize<'de>
{
    let value = serde_json::from_str(json)?;
    Ok(value)
}

pub fn from_json_file<T, P: AsRef<Path>>(file: P) -> Result<T, SerializationError> 
where T: for<'de> Deserialize<'de>, 
{
    let json = helpers::file::read_to_string(file)?;
    Ok(serde_json::from_str(&json)?)
}

/*
pub fn to_xml<T>(value: &T) -> Result<String, SerializationError>
where T: ?Sized + serde::Serialize,
{
    let mut buffer = String::new();
    let mut binding = quick_xml::se::Serializer::new(&mut buffer);
    binding
        .indent(' ', 4_usize);
    
    value.serialize(binding)?;
    let mut xml_decl = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n".to_owned();
    xml_decl.push_str(&buffer);
    Ok(xml_decl)
}

pub fn from_xml_str<'de, T>(s: &'de str) -> Result<T, SerializationError>
where T: Deserialize<'de>,
{
    let mut de = quick_xml::de::Deserializer::from_str(s);
    T::deserialize(&mut de)
        .map_err(|err| err.into())
}

pub fn from_xml_file<T, P: AsRef<Path>>(file: P) -> Result<T, SerializationError>  
where T: for<'de> Deserialize<'de>,
{
    let xml = helpers::file::read_to_string(file)?;
    Ok(quick_xml::de::from_str(&xml)?)
}

pub fn to_dat<T: ?Sized + serde::Serialize>(value: &T) -> Result<Vec<u8>, SerializationError> {
    let bytes = bincode::serialize(value)?;
    Ok(bytes)
}

pub fn from_bytes<'de, T>(bytes: &'de &[u8]) -> Result<T, SerializationError>
where T: Deserialize<'de> {
    Ok(bincode::deserialize(bytes)?)
}

pub fn from_dat_file<T, P: AsRef<Path>>(file: P) -> Result<T, SerializationError>  
where T: for<'de> Deserialize<'de>,
{
    let mut file = helpers::file::open(file)?;
    let mut buffer = Vec::new();
    helpers::file::read_to_end(&mut file, &mut buffer)?;
    Ok(bincode::deserialize(&buffer)?)
}
*/