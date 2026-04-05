use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures_dir = workspace_root.join("fixtures/valid");
    let examples_dir = workspace_root.join("examples");

    let dirs = vec![
        ("fixture", fixtures_dir.clone()),
        ("example", examples_dir.clone()),
    ];

    for (prefix, dir) in &dirs {
        if !dir.exists() {
            panic!("Directory not found: {}", dir.display());
        }
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "nanachi") {
                let stem = path.file_stem().unwrap().to_str().unwrap();
                let source = std::fs::read_to_string(&path).unwrap();

                let grammar = nanachi_meta::compile(&source)
                    .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));

                let tokens = nanachi_generator::generate(&grammar);

                // Pretty-print for debuggability
                let code = match syn::parse2::<syn::File>(tokens.clone()) {
                    Ok(file) => prettyplease::unparse(&file),
                    Err(_) => tokens.to_string(),
                };

                let out_file = Path::new(&out_dir).join(format!("{prefix}_{stem}.rs"));
                std::fs::write(&out_file, &code).unwrap();

                println!("cargo::rerun-if-changed={}", path.display());
            }
        }
    }
}
