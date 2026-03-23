# pathfinder

**[Live Demo](https://4rh1t3ct0r7.github.io/pathfinder/)**

Interactive maze visualizer and solver built with Rust (WASM) and SolidJS.

![Screenshot](docs/screenshot.png)

## Features

- **10 generation algorithms** -- DFS, Kruskal, Prim, Eller, Wilson, Growing Tree, Binary Tree, Sidewinder, Aldous-Broder, Hunt & Kill
- **8 solving algorithms** -- BFS, DFS, A*, Dijkstra, Greedy Best-First, Wall Follower, Tremaux, Dead-End Filling
- **Side-by-side comparison mode** -- run two solvers on the same maze and see which wins
- **Auto Compare All** -- benchmark all 8 solvers in one click with a sortable results table
- **Step-by-step animation** with adjustable speed, pause, and single-step controls
- **Real-time metrics** -- steps, visited cells, path length, dead ends, efficiency
- **Presets** -- quick-start maze configurations from 5x5 to 100x100
- **Export to PNG** -- high-resolution maze snapshots
- **Share via URL** -- copy a link that encodes maze parameters
- **i18n** -- English and Russian interface
- **Mobile responsive** -- works on phones and tablets
- **Zoom & pan** -- scroll to zoom, drag to pan, double-click to reset view

## Tech Stack

| Layer | Technology |
|-------|------------|
| Algorithms | Rust, compiled to WebAssembly |
| UI | SolidJS + TypeScript |
| Rendering | Canvas 2D (HiDPI-aware) |
| Build | Vite + wasm-pack |
| CI/CD | GitHub Actions, deployed to GitHub Pages |

## Quick Start

```bash
# 1. Build WASM module
cargo install wasm-pack
wasm-pack build crates/pathfinder-core --target web --out-dir ../../web/pkg

# 2. Install frontend dependencies
cd web
npm install

# 3. Start dev server
npx vite
```

Open http://localhost:5173 in your browser.

## Production Build

```bash
wasm-pack build crates/pathfinder-core --release --target web --out-dir ../../web/pkg
cd web && npm ci && npx vite build
```

The output will be in `web/dist/`.

## Project Structure

```
pathfinder/
  crates/pathfinder-core/    Rust library (maze engine, generators, solvers)
  web/
    src/
      components/            SolidJS UI components
      stores/                Reactive state management
      wasm/                  WASM bridge layer
      styles/                Global CSS
      i18n.ts                Internationalization (EN/RU)
      hooks/                 Shared URL utilities
    pkg/                     Generated WASM output (git-ignored)
```

## License

[MIT](LICENSE)
