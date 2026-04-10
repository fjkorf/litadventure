# Demo Walkthrough

Complete puzzle sequence from start to victory.

## The Study

You start in a quiet study, facing a desk, bookshelf, and a door.

**Objective: "Explore the study."**

1. **Look around.** Hover over objects to see them glow. Click the **bookshelf** — you'll get a description but nothing happens. This is just for atmosphere.

2. **Click the desk.** The camera smoothly zooms in. The objective completes, and a new one appears: **"Find the flashlight."**

3. **Click the drawer.** It slides open with a smooth animation. A flashlight appears inside.

4. **Click the flashlight.** It's collected into your inventory (visible in the right panel). The objective completes: **"Explore the hallway."**

5. **Press Escape or right-click** to back out to the room overview.

## The Hallway

6. **Click the door** on the right side of the study. The camera smoothly transitions to the hallway. The top panel now reads "The Hallway."

7. **Explore.** Click the **painting** — it has a clue written on the back: "42". Click the **lens** on the wall to pick it up. Click the **frame** on the floor to pick it up.

8. **Combine items.** With both Lens and Frame in your inventory, click the **Combine** button in the inventory panel. They merge into a **Magnifying Glass**.

**Objective: "Unlock the mysterious door."**

9. **Click the locked door.** Since you have the flashlight, you shine it on the lock. It clicks open. The objective completes: **"Escape!"**

10. **Click the unlocked door.** "You step through the door into the light." The victory screen appears.

## Controls Summary

| Input | Action |
|-------|--------|
| Left click | Interact with object |
| Right click / Escape | Go back to previous camera position |
| P | Pause (opens help overlay) |
| F1 | Toggle debug overlay |

## Hint System

If you're stuck, wait 30 seconds. A hint appears in the bottom panel (amber text). After 60 seconds, a more specific hint. After 90 seconds, the solution is told directly. Hints reset whenever you do something meaningful.

## Objects in the Demo

### Study
| Object | Components | Behavior |
|--------|-----------|----------|
| Desk | Clickable, NavigatesTo | Zooms camera to desk |
| Drawer | Clickable, ObjectState(Closed), TweenConfig | Opens/closes with animation, reveals flashlight |
| Flashlight | Clickable, InventoryItem, ContainedInName | Hidden until drawer opens, collectible |
| Bookshelf | Clickable | Description only |
| Door | Clickable, Portal | Transitions to hallway |

### Hallway
| Object | Components | Behavior |
|--------|-----------|----------|
| Painting | Clickable | Clue text |
| Lens | Clickable, InventoryItem | Collectible, combines with Frame |
| Frame | Clickable, InventoryItem | Collectible, combines with Lens |
| Locked Door | Clickable, ObjectState(Locked), RequiresItem | Needs flashlight to unlock, then triggers win |
| Door to Study | Clickable, Portal | Returns to study |
