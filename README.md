# LitAdventure

A framework for building MYST-inspired point-and-click adventure games using Bevy, egui, and litui.

Players explore 3D environments through static camera positions, clicking objects to examine them, collecting items, solving puzzles, and unlocking new areas. The engine is designed to be accessible to new players while evoking the atmosphere of classic adventure games.

## Quick Start

```sh
cargo build
cargo run
```

See [docs/getting-started.md](docs/getting-started.md) for prerequisites and detailed setup.

## Documentation

| Document | Description |
|----------|-------------|
| [Getting Started](docs/getting-started.md) | Build, run, test, project structure |
| [Architecture](docs/architecture.md) | Engine/data separation, module map, state and message flow |
| [Components](docs/components.md) | All game components with fields and usage |
| [Data Formats](docs/data-formats.md) | RON schemas for objectives, hints, recipes, rooms, level manifests, saves |
| [Blender Workflow](docs/blender-workflow.md) | bevy_skein setup, adding components, exporting glTF |
| [Adding Content](docs/adding-content.md) | How to add rooms, items, objectives, hints |
| [Debug Mode](docs/debug-mode.md) | F1 overlay, wireframe colors, camera spot visualization |
| [Demo Walkthrough](docs/demo-walkthrough.md) | Full puzzle sequence from start to victory |
| [Design Principles](docs/design-principles.md) | 8 UX principles from genre research |
| [Screenshot Testing](docs/screenshot-testing.md) | Visual regression test workflow |

## Related Projects

- [litui](https://github.com/fjkorf/litui) — Compile-time Markdown-to-egui UI framework
- [Bevy](https://bevyengine.org/) — Game engine
- [bevy_skein](https://bevyskein.dev/) — Blender-to-Bevy component pipeline
