use std::fmt;
use std::fs::OpenOptions;
use std::io::{Seek, Write};
use std::io::SeekFrom::Start;
use std::path::Path;

use clap::Parser;
use instally_core::helpers::serializer;
use instally_core::workloads::definitions::Product;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opt {
    // Configuration folder
    #[arg(short, long, default_value = "config")]
    config: String,
}

fn main(){
    match inner(Opt::parse()) {
        Ok(()) => {
            println!("Completed");
        }

        Err(err) => {
            println!("Failed to create Setup binary. Inner details; \n{:?}", err)
        }
    }
}

fn inner(args: Opt) -> Result<(), Error> {
    let query: Vec<u8> = vec![35, 35, 35, 47, 80, 65, 89, 76, 79, 65, 68, 47, 35, 35, 35];
    
    let config_dir = Path::new(&args.config);
    let product_path = config_dir.join("product.xml");

    let tmp_product = Product::read_template(product_path).unwrap();

    std::fs::copy("instally-scaffold.exe", "Setup.exe")?;
    let file = std::cell::RefCell::new(OpenOptions::new()
        .read(true).write(true)
        .open("Setup.exe")?
    );

    let query_str = query.iter().map(|f| format!("{:X}", f)).collect::<Vec<String>>().join(" ");
    println!("Query str: {}", query_str);

    let locs = patternscan::scan(std::io::Read::by_ref(&mut *file.borrow_mut()), &query_str)
        .map_err(|e| e.to_string())?;

    if let Some(loc) = locs.first() {
        let pos = *loc as u64;

        println!("Found to start of payload at position {}", pos as usize + query.len());

        let xml_str = serializer::to_xml(&tmp_product).unwrap();
        let xml = xml_str.as_bytes();

        file.borrow_mut().seek(Start(pos + query.len() as u64)).unwrap();
        file.borrow_mut().write(&xml)?; 

        return Ok(());
    }

    Err("Could not find payload sequence.".to_string().into())
}


#[derive(Debug)]
pub struct Error {
    details: String
}

impl Error {
    pub fn new(msg: &str) -> Error {
        Error { 
            details: msg.to_string()
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {}", self.details)
    }
}

impl std::error::Error for Error { }

impl From<String> for Error {
    fn from(msg: String) -> Error {
        Error::new(&msg)
    }
}

impl From<Box<dyn std::error::Error + 'static>> for Error {
    fn from(err: Box<dyn std::error::Error + 'static>) -> Error {
        Error::new(&err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::new(&format!("IO error: {:?}", err))
    }
}