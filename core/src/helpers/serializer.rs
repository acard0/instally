use std::{path::Path, io::Read};

use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
pub enum SerializationError {
    #[error("IO error {0}")]
    Io(#[from] std::io::Error),
    #[error("XML error {0}")]
    Xml(#[from] quick_xml::DeError),
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
    let mut file = std::fs::OpenOptions::new().read(true).open(file)?;
    let mut xml = String::new();  
    file.read_to_string(&mut xml)?;

    Ok(quick_xml::de::from_str(&xml)?)
}