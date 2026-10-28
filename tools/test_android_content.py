#!/usr/bin/env python3
"""Run archive safety and terrain upload scheduling tests without a game."""
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    root = ROOT / '.local/android/content-tests'
    (root / 'src').mkdir(parents=True, exist_ok=True)
    (root / 'Cargo.toml').write_text('''[package]
name = "android-content-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde_json = "1"
miniz_oxide = "0.8"
zip = { version = "2", default-features = false, features = ["deflate"] }
''', encoding='utf-8')
    lock=tomllib.loads((ROOT / 'Cargo.lock').read_text(encoding='utf-8'))
    version=next(p['version'] for p in lock['package'] if p['name']=='zip' and p['version'].startswith('2.'))
    manifest=root / 'Cargo.toml'
    manifest.write_text(manifest.read_text().replace('version = "2"',f'version = "={version}"'),encoding='utf-8')
    (root / 'src/lib.rs').write_text('''#![allow(dead_code)]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[path = "../../../../crates/horizon-client/src/content_files.rs"] mod content_files;
#[path = "../../../../crates/horizon-client/src/gfx/upload_budget.rs"] mod upload_budget;
#[path = "../../../../crates/horizon-client/src/zip.rs"] mod archive_reader;
#[path = "../../../../crates/horizon-client/src/android/search_queue.rs"] mod search_queue;
''', encoding='utf-8')
    subprocess.run(['cargo', 'test', '--manifest-path', str(root / 'Cargo.toml')],
                   cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
