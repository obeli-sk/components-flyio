use anyhow::Result;
use std::path::Path;
use wit_bindgen_rust::Opts;
use wit_parser::Resolve;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=wit/");

    let opts = Opts {
        generate_all: true,
        additional_derive_attributes: vec![
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string(),
        ],
        ..Default::default()
    };
    let mut generator = opts.build();
    let mut resolve = Resolve::default();
    let (pkg, _files) = resolve.push_path("wit")?;
    let main_packages = vec![pkg];
    let world = resolve.select_world(&main_packages, None)?;
    let mut files = Default::default();
    generator.generate(&resolve, world, &mut files)?;

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dst = Path::new(&out_dir).join("generated.rs");
    let (_name, contents) = files.iter().next().unwrap();
    let contents = String::from_utf8(contents.to_vec()).unwrap();

    // https://github.com/bytecodealliance/wit-bindgen/issues/1386
    // Configure serde: rename all enums to use kebab-case
    let re = regex::Regex::new(r"(pub\s+enum\s+\w+)").unwrap();
    let contents = re.replace_all(&contents, "#[serde(rename_all = \"kebab-case\")]\n$1");

    std::fs::write(&dst, contents.into_owned())?;
    Ok(())
}
