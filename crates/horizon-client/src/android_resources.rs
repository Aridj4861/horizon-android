//! Reconstruct copied vanilla resources from the user's verified official download.
use crate::Result;
use std::{fs, path::Path};

pub const WORLDGEN: &[(&str, &str)] = &[
    ("noises", "worldgen/noise"),
    ("densities", "worldgen/density_function"),
    ("rules", "worldgen/material_rule"),
    ("conditions", "worldgen/material_condition"),
    ("settings", "worldgen/noise_settings"),
    ("carvers", "worldgen/carver"),
    ("biomes", "worldgen/biome"),
    ("world_presets", "worldgen/world_preset"),
    ("features", "worldgen/feature"),
    ("placed_features", "worldgen/placed_feature"),
    ("state_providers", "worldgen/block_state_provider"),
    ("biome_tags", "tags/worldgen/biome"),
    ("feature_tags", "tags/worldgen/feature"),
    ("block_tags", "tags/block"),
    ("fluid_tags", "tags/fluid"),
    ("processor_lists", "worldgen/processor_list"),
    ("template_pools", "worldgen/template_pool"),
    ("structures", "worldgen/structure"),
    ("structure_sets", "worldgen/structure_set"),
];

fn walk(root: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for item in fs::read_dir(root)? {
        let path = item?.path();
        if path.is_dir() {
            walk(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn aggregate(root: &Path, source: &str, target: &str) -> Result<()> {
    let base = root.join("data/minecraft").join(source);
    let mut files = Vec::new();
    walk(&base, &mut files)?;
    files.sort();
    let mut values = serde_json::Map::new();
    for file in files {
        if file.extension().is_none_or(|v| v != "json") {
            continue;
        }
        let relative = file.strip_prefix(&base)?.with_extension("");
        values.insert(
            format!(
                "minecraft:{}",
                relative.to_string_lossy().replace('\\', "/")
            ),
            serde_json::Value::String(fs::read_to_string(file)?),
        );
    }
    let target = root.join(target);
    fs::create_dir_all(target.parent().ok_or("Missing resource parent")?)?;
    fs::write(target, serde_json::to_vec(&values)?)?;
    Ok(())
}

pub fn rebuild(root: &Path) -> Result<()> {
    for (name, source) in WORLDGEN {
        aggregate(root, source, &format!("worldgen/{name}.json"))?;
    }
    for (name, source) in [
        ("block-tags", "tags/block"),
        ("damage-tags", "tags/damage_type"),
        ("damage-types", "damage_type"),
    ] {
        aggregate(root, source, &format!("{name}.json"))?;
    }
    let base = root.join("data/minecraft/structure");
    let mut files = Vec::new();
    walk(&base, &mut files)?;
    for file in files {
        if file.extension().is_none_or(|v| v != "nbt") {
            continue;
        }
        let target = root
            .join("worldgen/structure/minecraft")
            .join(file.strip_prefix(&base)?);
        fs::create_dir_all(target.parent().ok_or("Missing structure parent")?)?;
        fs::copy(file, target)?;
    }
    Ok(())
}

/// The manifest contains only names, hashes and the pinned upstream revision.
/// PNGs remain outside our APK and source repository.
pub fn controls(root: &Path) -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../packaging/android/bedrock-controls.json"
    ))?;
    let revision = manifest["revision"]
        .as_str()
        .ok_or("Missing control revision")?;
    let target = root.join("assets/minecraft/textures/gui/bedrock");
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(std::time::Duration::from_secs(30))
        .timeout_read(std::time::Duration::from_secs(60))
        .build();
    for entry in manifest["files"]
        .as_array()
        .ok_or("Missing control files")?
    {
        let name = entry["path"].as_str().ok_or("Missing control path")?;
        let hash = entry["sha256"].as_str().ok_or("Missing control checksum")?;
        let size = entry["size"]
            .as_u64()
            .filter(|n| *n <= 1024 * 1024)
            .ok_or("Invalid control size")?;
        let relative = Path::new(name);
        if relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("Invalid control destination".into());
        }
        let file = target.join(relative);
        if fs::read(&file).is_ok_and(|data| {
            data.len() as u64 == size && format!("{:x}", Sha256::digest(data)) == hash
        }) {
            continue;
        }
        let source = if name == "LICENSE.md" {
            "LICENSE.md".into()
        } else {
            format!(
                "resource_pack/textures/{}",
                entry["source"].as_str().ok_or("Missing control source")?
            )
        };
        let url =
            format!("https://raw.githubusercontent.com/Mojang/bedrock-samples/{revision}/{source}");
        let mut bytes = Vec::new();
        agent
            .get(&url)
            .call()?
            .into_reader()
            .take(size + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != size || format!("{:x}", Sha256::digest(&bytes)) != hash {
            return Err(format!("Control texture checksum mismatch: {name}").into());
        }
        fs::create_dir_all(file.parent().ok_or("Missing texture parent")?)?;
        fs::write(file, bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temporary() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "horizon-android-data-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
    #[test]
    fn reconstructs_namespaced_resources_and_nested_structures() {
        let root = temporary();
        let source = root.join("data/minecraft/worldgen/noise/sub");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("test.json"), "{\"amplitude\":1}").unwrap();
        let source = root.join("data/minecraft/structure/village");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("test.nbt"), b"synthetic structure").unwrap();
        rebuild(&root).unwrap();
        let map: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("worldgen/noises.json")).unwrap()).unwrap();
        assert_eq!(map["minecraft:sub/test"], "{\"amplitude\":1}");
        assert_eq!(
            fs::read(root.join("worldgen/structure/minecraft/village/test.nbt")).unwrap(),
            b"synthetic structure"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[ignore = "requires the owner's pinned client fixture; optionally downloads controls"]
    fn official_client_rebuild_matches_prepared_resources() {
        use std::io::Read;
        let client = std::env::var_os("HORIZON_ANDROID_TEST_CLIENT").expect("client fixture");
        let prepared = std::path::PathBuf::from(
            std::env::var_os("HORIZON_TEST_RESOURCES").expect("prepared fixture"),
        );
        let root = temporary();
        let mut zip = zip::ZipArchive::new(fs::File::open(client).unwrap()).unwrap();
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).unwrap();
            if entry.is_dir() || !entry.name().starts_with("data/minecraft/") {
                continue;
            }
            let path = root.join(entry.enclosed_name().unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            fs::write(path, data).unwrap();
        }
        rebuild(&root).unwrap();
        for (name, _) in WORLDGEN {
            let path = format!("worldgen/{name}.json");
            let read = |base: &Path| -> serde_json::Value {
                serde_json::from_slice(&fs::read(base.join(&path)).unwrap()).unwrap()
            };
            assert_eq!(read(&root), read(&prepared), "{path}");
        }
        let mut templates = Vec::new();
        walk(&prepared.join("worldgen/structure"), &mut templates).unwrap();
        assert!(!templates.is_empty());
        for file in &templates {
            let relative = file.strip_prefix(&prepared).unwrap();
            assert_eq!(
                fs::read(root.join(relative)).unwrap(),
                fs::read(file).unwrap(),
                "{relative:?}"
            );
        }
        if std::env::var_os("HORIZON_TEST_DOWNLOAD_CONTROLS").is_some() {
            controls(&root).unwrap();
        }
        println!(
            "Matched {} worldgen maps and {} templates",
            WORLDGEN.len(),
            templates.len()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
