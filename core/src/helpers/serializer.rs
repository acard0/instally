use std::path::Path;

use serde::Deserialize;
use rust_i18n::error::*;
use convert_case::*;
use crate::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum SerializationError {
    #[error("io.{}", .0.kind().to_string().to_case(Case::Kebab))]
    Io(#[from] std::io::Error),

    #[error("xml-parse")]
    XmlParse(#[from] quick_xml::DeError),

    #[error("xml-serialize")]
    XmlSerialize(#[from] quick_xml::Error),
}

pub fn to_xml<T>(value: &T) -> Result<String, SerializationError>
where
    T: ?Sized + serde::Serialize,
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

pub fn from_str<'de, T>(s: &'de str) -> Result<T, SerializationError>
where
    T: Deserialize<'de>,
{
    let mut de = quick_xml::de::Deserializer::from_str(s);
    T::deserialize(&mut de)
        .map_err(|err| err.into())
}

pub fn from_file<T, P: AsRef<Path>>(file: P) -> Result<T, SerializationError>  
where
    T: for<'de> Deserialize<'de>,
{
    let xml = fs_err::read_to_string(&file)?;
    Ok(quick_xml::de::from_str(&xml)?)
}