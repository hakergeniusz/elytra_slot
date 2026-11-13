# Elytra Slot

A Horizon native mod: the survival inventory gains a dedicated **elytra slot**
beside the player model. Click it to wear an elytra and keep flying while your
chestplate is kept safe, and click again to swap the two back — the same deal
the original mod makes, without needing Curios, Trinkets or a server-side
install.

## What it does

- **A dedicated elytra slot** in the plain survival inventory screen, drawn
  right of the player model (above the offhand slot), with the vanilla slot
  frame, ghost icon, hover highlight and tooltip. While the chest slot holds
  an elytra — however it got there — the slot shows it.
- **One-click equip/unequip.** Click the slot with an elytra anywhere
  (cursor first, then the main inventory, then the hotbar) and it is worn;
  the chestplate that was in the chest slot is kept by the mod and named in
  the slot's tooltip. Click again and the elytra comes off and the kept
  chestplate is worn again by the same click. With a full inventory the swap
  still ends bare-handed (the elytra parks where the chestplate waited).
- **The flight is real.** The equipped elytra sits in the engine's real chest
  armor slot, so Horizon's own glide check (`canGlideUsing` over the worn
  pieces), the wing renderer, durability wear, death drops and every server
  see honest equipment. Nothing is faked: gliding works in single player, on
  LAN and on vanilla-protocol servers, with no server-side install — unlike
  the original, which needs Curios/Trinkets on both ends.
- **Self-healing state.** The elytra can leave the chest slot by outside
  means: a death (it drops like any worn armor), a vanilla click on the chest
  slot, another mod. The mod notices within a tick, lets its slot go, and the
  kept chestplate goes back on through the game's own menu clicks (on a
  server once the inventory is open, which is when clicks go out there).
- **Settings.** Options > Mods > Elytra Slot: an Enabled toggle and read-only
  rows saying what the slot holds and what chestpiece is kept.

## The one deviation from the original

[The original](https://github.com/illusivesoulworks/elytraslot) puts the elytra
in a Curios/Trinkets accessory slot, so the chestplate stays worn and visible
while you fly. Horizon's survival inventory has exactly one chest slot and no
accessory-slot system, so two chest items cannot be worn at once. This port
keeps the chestplate *stored* instead of visible: it is moved to a real
inventory slot, tracked by the mod, and automatically worn again when the
elytra is unequipped. Everything else works as upstream does.

## Port notes

- Clean-room reimplementation for Horizon, written from the original's public
  behavior and Horizon's own source. No original code or assets were copied.
- The mod is a client-side native mod (`.hnmod`): it hooks the inventory
  click path, the screen render hook and the game tick, and moves items only
  through the engine's own verified menu engine, so servers receive honest
  container clicks.
- The elytra it equips is any stack the engine's item table puts in the chest
  slot with the `glider` component (`minecraft:elytra` out of the box).

## Build

```
python3 -I tools/test.py          # format, clippy, the runtime's own tests
python3 -I tools/horizon.py check # staged: type-check against the pinned Horizon
python3 -I tools/horizon.py test  # staged: the runtime's tests in the game tree
python3 -I tools/package.py       # target/elytra_slot-0.1.0.hnmod
```

## Credits

- [Elytra Slot](https://github.com/illusivesoulworks/elytraslot) by
  [Illusive Soul Works](https://github.com/illusivesoulworks) — the original
  mod for Forge/NeoForge (Curios) and Fabric (Trinkets), LGPL-3.0-or-later.
  This project is an independent clean-room reimplementation of its idea for
  [Horizon](https://github.com/horizon-rust/horizon); no original code.
- Licensed MIT, see [LICENSE](LICENSE).

Copyright (c) 2026 hakergeniusz
