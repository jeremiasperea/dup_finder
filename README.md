# dup_finder

Herramienta CLI rápida para encontrar archivos duplicados en un árbol de directorios. Escrita en Rust.

## Características

- **Algoritmo de dos pasadas**: agrupa por tamaño primero, luego hashea solo candidatos (SHA-256)
- **Hasheo paralelo** con Rayon para aprovechar múltiples núcleos
- **Hash streaming** (buffer 64 KB) — maneja archivos grandes sin cargarlos enteros en memoria
- **Exclusiones inteligentes**: patrones glob (`--exclude`), ignora symlinks y archivos vacíos
- **Filtro por tamaño**: `--min-size MB` para saltar archivos chicos
- **Salida legible**: grupos ordenados por espacio desperdiciado (duplicados más grandes primero)

## Instalación

```bash
cargo install --path .
```

O compilar localmente:

```bash
cargo build --release
./target/release/dup_finder
```

## Uso

```bash
dup_finder [--min-size MB] [--exclude PATRÓN]...
```

### Opciones

| Flag | Descripción |
|------|-------------|
| `--min-size N` | Ignora archivos menores a N megabytes |
| `--exclude PATRÓN` | Excluye rutas que coincidan con el patrón glob (se puede repetir) |
| `<N>` (posicional) | Legacy: primer argumento numérico = min-size en MB |

### Ejemplos

```bash
# Escanear directorio actual
dup_finder

# Ignorar archivos menores a 10 MB
dup_finder --min-size 10

# Excluir artefactos de build y logs
dup_finder --exclude "target/**" --exclude "*.log" --exclude ".git/**"

# Combinar ambos
dup_finder --min-size 5 --exclude "node_modules/**" --exclude "*.tmp"
```

## Salida

```
Directorio base: /home/usuario/proyectos
Escaneando...

Archivos encontrados: 12450
Candidatos (mismo tamaño): 342 archivos en 87 grupos

=== DUPLICADOS (12 grupos) ===

[abc123def456] tamaño: 45.23 MB | 3 copias | desperdicio: 90.46 MB
    /home/usuario/proyectos/videos/clip.mp4
    /home/usuario/proyectos/backup/clip.mp4
    /home/usuario/proyectos/archivo/viejo/clip.mp4

[a1b2c3d4e5f6] tamaño: 12.00 MB | 2 copias | desperdicio: 12.00 MB
    /home/usuario/proyectos/imagenes/foto.jpg
    /home/usuario/proyectos/descargas/foto.jpg

Espacio desperdiciado total: 102.46 MB
Tiempo: 2.34s
```

## Cómo funciona

1. **Recorre** el árbol de directorios (ignora symlinks para evitar ciclos)
2. **Agrupa por tamaño** — archivos con tamaño único no pueden ser duplicados, se descartan sin hashear
3. **Hashea en paralelo** los candidatos con SHA-256 (streaming de 64 KB por chunk)
4. **Agrupa por hash** — hashes idénticos = contenido idéntico
5. **Reporta** ordenado por espacio desperdiciado descendente

## Requisitos

- Rust 1.70+ (edición 2021)

## Dependencias

- [`sha2`](https://crates.io/crates/sha2) — hashing SHA-256
- [`rayon`](https://crates.io/crates/rayon) — iteración paralela
- [`glob`](https://crates.io/crates/glob) — matching de patrones para exclusiones

## Licencia

MIT