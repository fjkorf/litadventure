# Design Principles

These principles were derived from genre research into MYST-like point-and-click adventure games. They guide every interaction and UI decision.

## The 8 Principles

### 1. Always Respond

Every click gets visual or textual feedback. Never silence. Silence is the #1 player frustration in adventure games.

- Clicking any `Clickable` entity always sets `FeedbackText`
- Failed item uses show a message ("It's too dark to see the lock clearly.")
- Invalid combinations show "These don't seem to work together."
- Camera transitions show "..." while moving

### 2. Contextual HUD

Minimal by default. UI elements appear when relevant and stay out of the way.

- Inventory panel is always visible but unobtrusive (right side, 200px)
- Objectives appear in the bottom bar
- Hints only appear after idle time
- Help overlay only on pause
- No central panel — the 3D scene fills the viewport

### 3. Concise Second-Person Text

"You open the drawer. A flashlight rests inside." Two to four sentences max. Every mentioned noun should be examinable. Avoid flowery padding.

This is the universal convention in interactive fiction. Second person ("you") is standard.

### 4. Visual Affordances

Interactive objects glow on hover. This prevents pixel hunting — the #1 frustration for newcomers to the genre.

- `on_hover_start`: Boosts emissive on the object's material
- `on_hover_end`: Restores original emissive
- Debug mode (F1) shows wireframe click zones color-coded by type

### 5. Progressive Hints

Optional timed hint system based on The Room's model (gold standard in the genre):

- **Tier 1** (30s idle): Vague nudge — "What catches your eye?"
- **Tier 2** (60s idle): Specific suggestion — "The desk might be worth a closer look."
- **Tier 3** (90s idle): Direct solution — "Click the desk to examine it."
- No penalty for hints
- Timer resets on any meaningful player action
- Thresholds configurable in `hints.ron`

### 6. Node-Based Navigation

Static camera positions with smooth transitions. The player clicks to move between fixed viewpoints, not free-roam.

- Matches the original MYST model
- Avoids motion sickness (a real concern cited by players)
- Makes interactive areas clearer (camera always frames the interesting objects)
- Back-navigation via Escape/right-click with a history stack

### 7. Examine-First Interaction

Click zooms/examines. A second action picks up or uses. The player always sees what they're interacting with before committing.

- Click desk -> camera zooms to desk (examine)
- Click drawer -> drawer opens (interact)
- Click flashlight -> collected (commit)
- This two-step flow prevents accidental actions

### 8. Journal System

Track discovered clues, visited areas, and current objectives. Players should always know what they're working toward.

Currently implemented as:
- Objectives panel (bottom bar) showing current goal
- Inventory panel (right side) showing collected items
- Feedback text showing last interaction result

Future enhancements:
- Full journal page tracking all discovered clues
- Map showing visited rooms
- Completed objectives history

## Genre Context

These principles address the top frustrations identified in the genre research:

| Frustration | Principle That Fixes It |
|-------------|------------------------|
| Pixel hunting (tiny hidden hotspots) | #4 Visual Affordances |
| Obscure puzzle logic | #5 Progressive Hints |
| Getting stuck with zero feedback | #1 Always Respond |
| No clear goals or direction | #8 Journal System |
| Motion sickness from free-roam | #6 Node-Based Navigation |
| Accidental item use | #7 Examine-First Interaction |

## Reference Games

These games informed the design:

| Game | What We Took |
|------|-------------|
| MYST (1993) | Node-based navigation, environmental storytelling |
| The Room (2012) | Timed progressive hints, item manipulation |
| Outer Wilds (2019) | Knowledge-based progression, Ship Log |
| Firmament (2023) | Universal tool, companion narrator |
| Riven Remake (2024) | Accessibility, screenshot annotation |
| Lorelei and the Laser Eyes (2024) | UI minimalism, non-linear puzzles |

## Accessibility

### Implemented
- **Rebindable controls** via `assets/settings/keybindings.ron` (hot-reloadable)
- **Tab navigation** through clickable objects with emissive focus highlight
- **Enter key** to confirm action on focused object (full keyboard-only play)
- **Click-to-select** inventory combine (gold border selection, no drag required)
- **Dwell-click** (hover-to-click) for motor accessibility (configurable duration, disabled by default)
- **EguiWantsInput guards** preventing input double-handling between UI and 3D scene
- **InputIntent abstraction** decoupling raw input from game logic (enables future gamepad/touch/switch access)

### Still Planned
- Colorblind modes (player-chosen colors, not just filters)
- Subtitle/font size options
- Difficulty options (hint frequency: off / patient / normal / eager)
- Gamepad support via InputIntent layer
- Switch access (auto-scan + single-button confirm)
- Screen reader integration via egui AccessKit
