# Screenshot Testing

Visual regression tests verify the 3D scene renders as expected.

## Workflow

```sh
# Generate a reference screenshot (do this once, or after intentional visual changes)
cargo run --example screenshot_test -- --save

# Compare current render against reference
cargo run --example screenshot_test
```

## How It Works

1. Opens a 1280x720 window (Retina: 2560x1440 actual pixels)
2. Spawns the procedural test scene (same geometry as the game, no UI overlay)
3. Waits 10 frames for GPU pipeline warm-up
4. Captures the rendered frame via `Screenshot::primary_window()`
5. In `--save` mode: writes to `tests/screenshots/scene_reference.png`
6. In compare mode: loads reference, compares per-pixel with tolerance

## Comparison Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| Window size | 1280x720 | Fixed for determinism |
| Warmup frames | 10 | Skipped before capture |
| Channel tolerance | 5/255 | Per-channel RGB difference allowed |
| Max diff percent | 2.0% | Percentage of pixels that can differ |
| Reference path | `tests/screenshots/scene_reference.png` | Checked into version control |
| Diff output | `tests/screenshots/scene_diff.png` | Saved on failure for inspection |

## When to Update the Reference

Update the reference (`--save`) after any intentional change to:
- Scene geometry or object positions
- Lighting (intensity, position, shadows)
- Materials or colors
- Camera position

Do NOT update after unintentional changes — investigate the diff first.

## CI Considerations

Screenshot tests require a GPU. Options for CI:
- **SwiftShader** (Vulkan software rasterizer) — set `VK_ICD_FILENAMES` env var
- **Machine with GPU** — most CI providers offer GPU instances
- The `WGPU_BACKEND` env var can force a specific backend

GPU rendering is not perfectly deterministic across drivers. The 2% tolerance threshold accounts for minor driver differences. Pin SwiftShader version for CI stability.

## Files

- `examples/screenshot_test.rs` — the test binary
- `tests/screenshots/scene_reference.png` — reference image (committed)
- `tests/screenshots/scene_diff.png` — captured image on failure (gitignored)
