fn main() {

    {
        let thmanyah = "assets/fonts/Thmanyah/thmanyahsans-Medium.otf";
        let fallback = "assets/fonts/fallback/IBMPlexSansArabic-Medium.ttf";
        let src = if std::path::Path::new(thmanyah).exists() {
            thmanyah
        } else {
            fallback
        };
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        std::fs::copy(src, out.join("arabic-font.bin")).unwrap();
        println!("cargo:rerun-if-changed={thmanyah}");
        println!("cargo:rerun-if-changed={fallback}");
    }

    {
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let dir = std::path::Path::new("assets/people");
        for (slot, stem) in ["Rayan", "Muayid"].iter().enumerate() {
            let dest = out.join(format!("person{slot}.png"));
            let found = std::fs::read_dir(dir).ok().and_then(|entries| {
                entries.flatten().find(|e| {
                    e.path()
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.eq_ignore_ascii_case(stem))
                })
            });
            match found {
                Some(entry) => {
                    let img = image::open(entry.path()).unwrap_or_else(|e| {
                        panic!("assets/people/{stem}: {e}");
                    });
                    let side = img.width().min(img.height());
                    let x = (img.width() - side) / 2;
                    let y = (img.height() - side) / 2;
                    let square = image::imageops::crop_imm(&img, x, y, side, side).to_image();
                    let target = side.min(512);
                    let scaled = image::imageops::resize(
                        &square,
                        target,
                        target,
                        image::imageops::FilterType::Lanczos3,
                    );
                    scaled.save(&dest).unwrap();
                }
                None => {
                    std::fs::write(&dest, []).unwrap();
                }
            }
        }
        println!("cargo:rerun-if-changed=assets/people");
    }

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap().replace('\\', "/");
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let mut parts: Vec<String> = version.split('.').map(str::to_owned).collect();
        parts.resize(4, "0".to_owned());

        let rc = std::fs::read_to_string("assets/windows/badeel.rc")
            .unwrap()
            .replace("@VER_COMMA@", &parts.join(","))
            .replace("@VER@", &parts.join("."))
            .replace("@ASSETS@", &format!("{manifest_dir}/assets/windows"));

        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("badeel.rc");
        std::fs::write(&out, rc).unwrap();
        embed_resource::compile(&out, embed_resource::NONE)
            .manifest_optional()
            .unwrap();
        println!("cargo:rerun-if-changed=assets/windows");
    }
}
