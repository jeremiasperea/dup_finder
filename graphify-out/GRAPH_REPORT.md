# Graph Report - .  (2026-07-23)

## Corpus Check
- Corpus is ~1,078 words - fits in a single context window. You may not need a graph.

## Summary
- 11 nodes · 14 edges · 3 communities (2 shown, 1 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Core Utilities
- File Walker
- Skill Registry

## God Nodes (most connected - your core abstractions)
1. `hash_file()` - 5 edges
2. `walk_dir()` - 5 edges
3. `main()` - 3 edges
4. `format_size()` - 2 edges
5. `Skill Registry` - 0 edges

## Surprising Connections (you probably didn't know these)
- `main()` --calls--> `hash_file()`  [EXTRACTED]
  src/main.rs → src/main.rs  _Bridges community 0 → community 1_

## Import Cycles
- None detected.

## Communities (3 total, 1 thin omitted)

### Community 0 - "Core Utilities"
Cohesion: 0.47
Nodes (5): Path, Result, format_size(), hash_file(), String

### Community 1 - "File Walker"
Cohesion: 0.50
Nodes (4): PathBuf, main(), walk_dir(), Vec

## Knowledge Gaps
- **1 isolated node(s):** `Skill Registry`
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `walk_dir()` connect `File Walker` to `Core Utilities`?**
  _High betweenness centrality (0.341) - this node is a cross-community bridge._
- **Why does `hash_file()` connect `Core Utilities` to `File Walker`?**
  _High betweenness centrality (0.291) - this node is a cross-community bridge._
- **Why does `main()` connect `File Walker` to `Core Utilities`?**
  _High betweenness centrality (0.061) - this node is a cross-community bridge._
- **What connects `Skill Registry` to the rest of the system?**
  _1 weakly-connected nodes found - possible documentation gaps or missing edges._