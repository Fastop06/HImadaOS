use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader, Cursor, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use ruzstd::decoding::StreamingDecoder;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: himada-pkg-extract <package.pkg.tar.[xz|zst]> [target_dir]");
        std::process::exit(1);
    }

    let pkg_path = &args[1];
    let target_root = if args.len() >= 3 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from("/")
    };

    if let Err(e) = extract_package(pkg_path, &target_root) {
        eprintln!("himada-pkg-extract error: {}", e);
        std::process::exit(1);
    }
}

fn extract_package(pkg_path: &str, target_root: &Path) -> io::Result<()> {
    let raw_file = File::open(pkg_path)?;
    let mut f = BufReader::new(raw_file);
    let mut magic = [0u8; 6];
    let n = f.read(&mut magic)?;
    if n < 4 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "File too small"));
    }

    use std::io::Seek;
    f.seek(io::SeekFrom::Start(0))?;

    let mut decompressed_tar: Vec<u8> = Vec::new();

    if magic.starts_with(&[0xFD, b'7', b'z', b'X', b'Z', 0x00]) {
        // XZ archive
        lzma_rs::xz_decompress(&mut f, &mut decompressed_tar)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("XZ error: {}", e)))?;
    } else if magic.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        // Zstandard archive
        let mut decoder = StreamingDecoder::new(&mut f)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Zstd error: {:?}", e)))?;
        decoder.read_to_end(&mut decompressed_tar)?;
    } else if magic.starts_with(&[0x1F, 0x8B]) {
        // Gzip archive - fallback to raw read or tar
        f.read_to_end(&mut decompressed_tar)?;
    } else {
        // Assume uncompressed tar
        f.read_to_end(&mut decompressed_tar)?;
    }

    let cursor = Cursor::new(decompressed_tar);
    let mut archive = tar::Archive::new(cursor);
    let mut extracted_count = 0usize;
    let mut pending_symlinks: Vec<(PathBuf, PathBuf)> = Vec::new();

    for entry_res in archive.entries()? {
        let mut entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = match entry.path() {
            Ok(p) => p.to_path_buf(),
            Err(_) => continue,
        };

        let path_str = path.to_string_lossy();
        let clean = path_str.trim_start_matches('.').trim_start_matches('/');
        if clean.is_empty()
            || clean.starts_with(".PKGINFO")
            || clean.starts_with(".BUILDINFO")
            || clean.starts_with(".MTREE")
            || clean.starts_with(".INSTALL")
        {
            continue;
        }

        let dest_path = target_root.join(clean);

        if entry.header().entry_type().is_dir() {
            let _ = fs::create_dir_all(&dest_path);
            continue;
        }

        if let Some(parent) = dest_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if entry.header().entry_type().is_symlink() {
            if let Ok(Some(link_target)) = entry.link_name() {
                pending_symlinks.push((dest_path, link_target.to_path_buf()));
            }
            continue;
        }

        // Read file contents
        let mut content = Vec::new();
        if entry.read_to_end(&mut content).is_ok() {
            let is_elf = content.starts_with(b"\x7fELF");
            let mode = entry.header().mode().unwrap_or(0o644);
            let is_exec = is_elf || (mode & 0o111) != 0 || clean.starts_with("usr/bin/") || clean.starts_with("bin/");

            let final_mode = if is_exec { 0o755 } else { 0o644 };

            if let Ok(mut out_file) = File::create(&dest_path) {
                let _ = out_file.write_all(&content);
                let _ = out_file.set_permissions(fs::Permissions::from_mode(final_mode));
                extracted_count += 1;
            }

            // If binary is in usr/bin/, ensure it's also reachable in /bin/
            if clean.starts_with("usr/bin/") {
                let bin_name = &clean["usr/bin/".len()..];
                let alt_path = target_root.join("bin").join(bin_name);
                if let Some(parent) = alt_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(mut out_alt) = File::create(&alt_path) {
                    let _ = out_alt.write_all(&content);
                    let _ = out_alt.set_permissions(fs::Permissions::from_mode(0o755));
                }
            } else if clean.starts_with("bin/") {
                let bin_name = &clean["bin/".len()..];
                let alt_path = target_root.join("usr/bin").join(bin_name);
                if let Some(parent) = alt_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(mut out_alt) = File::create(&alt_path) {
                    let _ = out_alt.write_all(&content);
                    let _ = out_alt.set_permissions(fs::Permissions::from_mode(0o755));
                }
            }

            // If library is in usr/lib/, ensure it's also in /lib/
            if clean.starts_with("usr/lib/") {
                let lib_name = &clean["usr/lib/".len()..];
                let alt_path = target_root.join("lib").join(lib_name);
                if let Some(parent) = alt_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(mut out_alt) = File::create(&alt_path) {
                    let _ = out_alt.write_all(&content);
                    let _ = out_alt.set_permissions(fs::Permissions::from_mode(0o755));
                }
            }
        }
    }

    // Resolve and extract symlinks
    for (dest_path, link_target) in pending_symlinks {
        let _ = fs::remove_file(&dest_path);
        let mut created = false;
        #[cfg(unix)]
        {
            if std::os::unix::fs::symlink(&link_target, &dest_path).is_ok() {
                created = true;
            }
        }
        if !created {
            let resolved = if link_target.is_relative() {
                dest_path.parent().map(|p| p.join(&link_target))
            } else {
                let stripped = link_target.strip_prefix("/").unwrap_or(&link_target);
                Some(target_root.join(stripped))
            };
            if let Some(res_path) = resolved {
                if res_path.exists() {
                    if fs::copy(&res_path, &dest_path).is_ok() {
                        let _ = fs::set_permissions(&dest_path, fs::Permissions::from_mode(0o755));
                        extracted_count += 1;
                        created = true;
                    }
                }
            }
        }

        if created {
            if let Ok(clean) = dest_path.strip_prefix(target_root) {
                let clean_str = clean.to_string_lossy();
                if clean_str.starts_with("usr/bin/") {
                    let bin_name = &clean_str["usr/bin/".len()..];
                    let alt_path = target_root.join("bin").join(bin_name);
                    if !alt_path.exists() && dest_path.exists() {
                        let _ = fs::copy(&dest_path, &alt_path);
                        let _ = fs::set_permissions(&alt_path, fs::Permissions::from_mode(0o755));
                    }
                } else if clean_str.starts_with("bin/") {
                    let bin_name = &clean_str["bin/".len()..];
                    let alt_path = target_root.join("usr/bin").join(bin_name);
                    if !alt_path.exists() && dest_path.exists() {
                        let _ = fs::copy(&dest_path, &alt_path);
                        let _ = fs::set_permissions(&alt_path, fs::Permissions::from_mode(0o755));
                    }
                } else if clean_str.starts_with("usr/lib/") {
                    let lib_name = &clean_str["usr/lib/".len()..];
                    let alt_path = target_root.join("lib").join(lib_name);
                    if !alt_path.exists() && dest_path.exists() {
                        let _ = fs::copy(&dest_path, &alt_path);
                        let _ = fs::set_permissions(&alt_path, fs::Permissions::from_mode(0o755));
                    }
                }
            }
        }
    }

    println!(":: Package extraction complete: {} files installed", extracted_count);
    Ok(())
}
