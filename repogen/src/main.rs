
use std::path::Path;

use clap::{arg, Parser};
use instally_core::{workloads::definitions::{Package, Repository, Product}, archiving};
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
    let opt = Opt::parse();
    
    let source_dir = Path::new(&opt.source);
    let target_dir = Path::new(&opt.output);
    let config_dir = Path::new(&opt.config);
    let product_path = config_dir.join("product.xml");

    let product = Product::read_file(product_path).unwrap();
    let mut repository = Repository::new(&product.name);

    let repository_packages_dir = target_dir.join("packages");
    std::fs::create_dir_all(repository_packages_dir.clone()).unwrap();

    let walkdir = WalkDir::new(source_dir).max_depth(1);
    let it = walkdir.into_iter().skip(1); 

    for package_folder in it {
        let package_folder = package_folder.unwrap();

        let package_dir = package_folder.path();
        let data_dir = package_dir.join("data");
        let meta_dir = package_dir.join("meta");

        let package_xml_path = meta_dir.join("package.xml");
        let package = Package::from_file(&package_xml_path).unwrap();

        repository.packages.push(package.clone());

        archiving::zip_write::compress_dir(
            &data_dir,
            &repository_packages_dir.join(format!("{}.zip", package.name)),
            zip::CompressionMethod::Bzip2
        ).unwrap();

        if !package.script.is_empty() {
            let package_script_path = meta_dir.join(package.script.clone());
            std::fs::copy(package_script_path, repository_packages_dir.join(package.script)).unwrap();
        }
    }

    let repository_xml = quick_xml::se::to_string(&repository).unwrap();
    std::fs::write(target_dir.join("repository.xml"), repository_xml).unwrap();

    if !product.script.is_empty() {
        let global_script_path = config_dir.join(product.script.clone());
        std::fs::copy(global_script_path, target_dir.join(product.script)).unwrap();
    }
}
