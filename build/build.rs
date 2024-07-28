use endpoint_gen::Data;
use std::env;
use std::path::PathBuf;

mod def;
fn main() -> eyre::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=./def");
    println!("cargo:rerun-if-changed=../docs/error_codes/error_codes.json");
    let mut root = env::current_dir()?;
    loop {
        if root.join(".cargo").exists() {
            break;
        }
        root = root.parent().unwrap().to_owned();
    }
    let root = root.to_str().unwrap();
    let dir = format!("{}/build", root);

    let data = Data {
        project_root: PathBuf::from(root),
        output_dir: PathBuf::from(&dir),
        services: def::service::get_services(),
        enums: def::enums::get_enums(),
        pg_funcs: def::service::get_proc_functions(),
    };
    endpoint_gen::main(data)?;
    Ok(())
}
