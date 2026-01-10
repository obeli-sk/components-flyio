use anyhow::Result;
use wit_bindgen_rust::Opts;

fn main() -> Result<()> {
    let path = Opts {
        generate_all: true,
        additional_derive_attributes: vec![
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string(),
        ],
        ..Default::default()
    }
    .build()
    .generate_to_out_dir(None)?;

    let contents = std::fs::read_to_string(&path)?;
    let re = regex::Regex::new(r"(pub\s+enum\s+\w+)").unwrap();
    let contents = re
        .replace_all(&contents, "#[serde(rename_all = \"kebab-case\")]\n$1")
        .into_owned();
    std::fs::write(&path, contents)?;

    Ok(())
}
