---
page:
  name: Inventory
  label: Items
  panel: right
  width: 200
  open: show_inventory
---

## Items ::title

::: foreach inv_items

| {name} | {qty} |
|--------|-------|

:::

[button](Combine){on_combine}

[display](inv_hint) ::muted
