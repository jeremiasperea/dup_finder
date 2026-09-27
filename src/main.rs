use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

use glob::Pattern;
use rayon::prelude::*;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
struct FileInfo {
    path: PathBuf,
    size: u64,
}

fn hash_file(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(65536, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>, exclude_patterns: &[Pattern]) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Sin acceso a {}: {}", dir.display(), e);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error al leer entrada: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Check exclusion patterns against relative path from root
        let rel_path = path.strip_prefix(std::env::current_dir().unwrap()).unwrap_or(&path);
        let rel_str = rel_path.to_string_lossy();
        if exclude_patterns.iter().any(|p| p.matches(&rel_str)) {
            continue;
        }

        // Ignorar symlinks para evitar ciclos infinitos.
        if path.is_symlink() {
            continue;
        }

        if path.is_dir() {
            walk_dir(&path, files, exclude_patterns);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

fn main() {
    let start = Instant::now();

    let root = std::env::current_dir().expect("No se puede obtener el directorio actual");
    println!("Directorio base: {}", root.display());
    println!("Escaneando...\n");

    // Parse CLI args: --min-size <MB> --exclude <pattern> --exclude <pattern> ...
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut min_bytes: u64 = 0;
    let mut exclude_patterns: Vec<Pattern> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--min-size" => {
                if i + 1 < args.len() {
                    if let Ok(mb) = args[i + 1].parse::<u64>() {
                        min_bytes = mb * 1024 * 1024;
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--exclude" => {
                if i + 1 < args.len() {
                    if let Ok(pat) = Pattern::new(&args[i + 1]) {
                        exclude_patterns.push(pat);
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                // Legacy: first positional arg = min-size in MB
                if min_bytes == 0 {
                    if let Ok(mb) = args[i].parse::<u64>() {
                        min_bytes = mb * 1024 * 1024;
                    }
                }
                i += 1;
            }
        }
    }

    if min_bytes > 0 {
        println!("Ignorando archivos menores a {} MB\n", min_bytes / 1024 / 1024);
    }
    if !exclude_patterns.is_empty() {
        println!("Excluyendo patrones: {:?}\n", exclude_patterns.iter().map(|p| p.as_str()).collect::<Vec<_>>());
    }

    let mut all_files: Vec<PathBuf> = Vec::new();
    walk_dir(&root, &mut all_files, &exclude_patterns);
    println!("Archivos encontrados: {}", all_files.len());

    // Primera pasada: agrupar por tamaño y cachear tamaños.
    // Archivos con tamaño único no pueden ser duplicados; se descartan sin hashear.
    let mut by_size: HashMap<u64, Vec<FileInfo>> = HashMap::new();
    for path in all_files {
        match fs::metadata(&path) {
            Ok(meta) => {
                let size = meta.len();
                // Ignorar archivos vacíos: técnicamente todos son "iguales"
                // pero reportarlos no tiene utilidad práctica.
                if size > min_bytes {
                    by_size.entry(size).or_default().push(FileInfo { path, size });
                }
            }
            Err(e) => eprintln!("Metadata error en {}: {}", path.display(), e),
        }
    }

    let size_groups: Vec<Vec<FileInfo>> = by_size.into_values().filter(|v| v.len() > 1).collect();
    let candidate_count: usize = size_groups.iter().map(|v| v.len()).sum();

    println!(
        "Candidatos (mismo tamaño): {} archivos en {} grupos",
        candidate_count,
        size_groups.len()
    );

    // Segunda pasada: hashear solo los candidatos (en paralelo).
    let mut by_hash: HashMap<String, Vec<FileInfo>> = HashMap::new();
    let mut hash_errors = 0usize;

    for group in size_groups {
        let results: Vec<_> = group
            .par_iter()
            .map(|info| (info.clone(), hash_file(&info.path)))
            .collect();

        for (info, result) in results {
            match result {
                Ok(hash) => {
                    by_hash.entry(hash).or_default().push(info);
                }
                Err(e) => {
                    eprintln!("Error al hashear {}: {}", info.path.display(), e);
                    hash_errors += 1;
                }
            }
        }
    }

    if hash_errors > 0 {
        eprintln!("\nArchivos no procesados por error: {}", hash_errors);
    }

    let mut duplicates: Vec<(String, Vec<FileInfo>)> = by_hash
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .collect();

    // Ordenar por tamaño descendente (los más relevantes primero) - usa tamaño cacheado.
    duplicates.sort_by(|(_, a), (_, b)| {
        let size_a = a[0].size;
        let size_b = b[0].size;
        size_b.cmp(&size_a)
    });

    let elapsed = start.elapsed();

    println!();

    if duplicates.is_empty() {
        println!("No se encontraron archivos duplicados.");
    } else {
        println!("=== DUPLICADOS ({} grupos) ===\n", duplicates.len());

        let mut total_wasted: u64 = 0;

        for (hash, infos) in &duplicates {
            let size = infos[0].size;
            let wasted = size * (infos.len() as u64 - 1);
            total_wasted += wasted;

            println!(
                "[{}] tamaño: {} | {} copias | desperdicio: {}",
                &hash[..12],
                format_size(size),
                infos.len(),
                format_size(wasted)
            );
            for info in infos {
                println!("    {}", info.path.display());
            }
            println!();
        }

        println!("Espacio desperdiciado total: {}", format_size(total_wasted));
    }

    println!("Tiempo: {:.2?}", elapsed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn hash_file_known_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let hash = hash_file(&file_path).unwrap();
        // SHA-256 de "hello world"
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn hash_file_empty() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        File::create(&file_path).unwrap();

        let hash = hash_file(&file_path).unwrap();
        // SHA-256 de string vacío
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn hash_file_large_streaming() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("large.bin");
        let mut file = File::create(&file_path).unwrap();
        // Escribir 200 KB (más que el buffer de 64 KB) para probar streaming
        let chunk = vec![0x42u8; 65536];
        for _ in 0..4 {
            file.write_all(&chunk).unwrap();
        }
        file.flush().unwrap();

        let hash = hash_file(&file_path).unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn walk_dir_collects_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir(root.join("subdir")).unwrap();
        File::create(root.join("a.txt")).unwrap();
        File::create(root.join("subdir").join("b.txt")).unwrap();
        // Symlink debería ser ignorado
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("a.txt", root.join("link.txt")).unwrap();
        }

        let mut files = Vec::new();
        walk_dir(root, &mut files, &[]);

        assert_eq!(files.len(), 2);
        let names: Vec<_> = files.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));
        // Symlink no debería aparecer
        assert!(!names.contains(&"link.txt"));
    }

    #[test]
    fn walk_dir_skips_symlink_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir(root.join("real_dir")).unwrap();
        File::create(root.join("real_dir").join("file.txt")).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("real_dir", root.join("link_dir")).unwrap();
        }

        let mut files = Vec::new();
        walk_dir(root, &mut files, &[]);

        // Solo el archivo dentro de real_dir, no los de link_dir (evita duplicados por symlink)
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn format_size_units() {
        assert_eq!(format_size(0), "0 bytes");
        assert_eq!(format_size(512), "512 bytes");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn duplicate_detection_basic() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Crear dos archivos con mismo contenido
        let mut f1 = File::create(root.join("dup1.txt")).unwrap();
        f1.write_all(b"same content").unwrap();
        let mut f2 = File::create(root.join("dup2.txt")).unwrap();
        f2.write_all(b"same content").unwrap();

        // Archivo único
        File::create(root.join("unique.txt")).unwrap();

        let mut all_files = Vec::new();
        walk_dir(root, &mut all_files, &[]);

        let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for path in all_files {
            let meta = fs::metadata(&path).unwrap();
            if meta.len() > 0 {
                by_size.entry(meta.len()).or_default().push(path);
            }
        }

        let size_groups: Vec<_> = by_size.into_values().filter(|v| v.len() > 1).collect();
        assert_eq!(size_groups.len(), 1);
        assert_eq!(size_groups[0].len(), 2);
    }
}
