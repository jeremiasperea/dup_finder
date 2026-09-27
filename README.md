# dup_finder

Fast CLI tool to find duplicate files in a directory tree. Written in Rust.

## Features

- **Two-pass algorithm**: groups by size first, then hashes only candidates (SHA-256)
- **Parallel hashing** with Rayon for multi-core speed
- **Streaming hash** (64 KB buffer) — handles large files without loading them entirely into memory
- **Smart exclusions**: glob patterns (`--exclude`), skips symlinks, ignores empty files
- **Size filter**: `--min-size MB` to skip tiny files
- **Human output**: groups sorted by wasted space (largest duplicates first)

## Install

```bash
cargo install --path .
```

Or build locally:

```bash
cargo build --release
./target/release/dup_finder
```

## Usage

```bash
dup_finder [--min-size MB] [--exclude PATTERN]...
```

### Options

| Flag | Description |
|------|-------------|
| `--min-size N` | Ignore files smaller than N megabytes |
| `--exclude PATTERN` | Exclude paths matching glob pattern (can repeat) |
| `<N>` (positional) | Legacy: first numeric argument = min-size in MB |

### Examples

```bash
# Scan current directory
dup_finder

# Ignore files under 10 MB
dup_finder --min-size 10

# Exclude build artifacts and logs
dup_finder --exclude "target/**" --exclude "*.log" --exclude ".git/**"

# Combine both
dup_finder --min-size 5 --exclude "node_modules/**" --exclude "*.tmp"
```

## Output

```
Directorio base: /home/user/projects
Escaneando...

Archivos encontrados: 12450
Candidatos (mismo tamaño): 342 archivos en 87 grupos

=== DUPLICADOS (12 grupos) ===

[abc123def456] tamaño: 45.23 MB | 3 copias | desperdicio: 90.46 MB
    /home/user/projects/videos/clip.mp4
    /home/user/projects/backup/clip.mp4
    /home/user/projects/archive/old/clip.mp4

[a1b2c3d4e5f6] tamaño: 12.00 MB | 2 copias | desperdicio: 12.00 MB
    /home/user/projects/images/photo.jpg
    /home/user/projects/downloads/photo.jpg

Espacio desperdiciado total: 102.46 MB
Tiempo: 2.34s
```

## How it works

1. **Walk** the directory tree (skips symlinks to avoid cycles)
2. **Group by size** — files with unique sizes cannot be duplicates, discarded without hashing
3. **Parallel hash** candidates with SHA-256 (streaming 64 KB chunks)
4. **Group by hash** — identical hashes = identical content
5. **Report** sorted by wasted space descending

## Requirements

- Rust 1.70+ (2021 edition)

## Dependencies

- [`sha2`](https://crates.io/crates/sha2) — SHA-256 hashing
- [`rayon`](https://crates.io/crates/rayon) — parallel iteration
- [`glob`](https://crates.io/crates/glob) — pattern matching for exclusions

## License

MIT