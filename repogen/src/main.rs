
use std::{path::Path, fs};

use clap::Parser;
use instally_core::{archiving, definitions::{package::{Package, PackageDefinition}, product::Product, repository::Repository}, helpers::serializer};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opt {
    // Source folder for packages
    #[arg(short, long, default_value = "source")]
    source: String,
    
    // Output folder for generated repository
    #[arg(short, long, default_value = "repository")]
    output: String,
    
    // Configuration folder
    #[arg(short, long, default_value = "config")]
    config: String,
}

fn main() {
    let rust_log = std::env::var("RUST_LOG").unwrap_or("info".into()); 
    std::env::set_var("RUST_LOG", rust_log);  
    env_logger::init();

    let opt = Opt::parse();
    
    let source_dir = Path::new(&opt.source);
    let target_dir = Path::new(&opt.output);
    let config_dir = Path::new(&opt.config);
    let product_path = config_dir.join("product.json");

    let tmp_product = Product::read_template(product_path).unwrap();
    let mut repository = Repository::new(&tmp_product.name, 0);

    let repository_packages_dir = target_dir.join("packages");
    std::fs::create_dir_all(repository_packages_dir.clone()).unwrap();

    let walkdir = WalkDir::new(source_dir).max_depth(1);
    let it = walkdir.into_iter().skip(1); 

    for package_folder in it {
        let package_folder = package_folder.unwrap();

        let package_dir = package_folder.path();
        let data_dir = package_dir.join("data");
        let meta_dir = package_dir.join("meta");
   
        let package_meta_path = meta_dir.join("package.json");
        let package_definition = PackageDefinition::from_file(&package_meta_path).unwrap();

        let archive_name = format!("{}{}.zip", package_definition.name,  package_definition.version);
        let archive_path = repository_packages_dir.join(&archive_name);
        let mut sha1 = String::new();

        log::info!("compressing {:?} package, destination {:?}", &package_definition.name, &archive_path);

        archiving::zip_write::compress_dir(
            &data_dir,
            &archive_path,
            zip::CompressionMethod::Bzip2,
            Some(&mut sha1),
            true
        ).unwrap();
        
        let size = fs::metadata(&archive_path).unwrap().len();
        let package = Package::from_definition(&package_definition, &archive_name, size, &sha1, &package_definition.script);
        repository.packages.push(package.clone());
        repository.size += size;

        if !package.script.is_empty() {
            let package_script_path = meta_dir.join(package.script.clone());
            std::fs::copy(package_script_path, repository_packages_dir.join(package.script)).unwrap();
        }

        log::info!("package file {:?} with sha1 {:?} created.", &archive_name, sha1);
    }

    if !tmp_product.script.is_empty() {
        repository.script = tmp_product.script.clone();
        let global_script_path = config_dir.join(tmp_product.script.clone());
        std::fs::copy(global_script_path, target_dir.join(tmp_product.script)).unwrap();
    }

    let repository_meta = serializer::to_json(&repository).unwrap();
    std::fs::write(target_dir.join("repository.json"), repository_meta).unwrap(); // TODO: create consts for file names

    log::info!("done");
}
