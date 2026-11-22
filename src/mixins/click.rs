// A click on the survival inventory that lands on the mod's slot: the elytra
// moves between the cursor and the bay here, as direct state — the bay is the
// mod's own, so no menu clicks or packets exist to make. In single player and
// on LAN that IS the shared state (the client hosts the world); on a remote
// server the slot stays read-only. An elytra on the cursor stores, and the
// bay's own stack (if any) comes to the cursor, so no item is ever lost.
// The vanilla click is skipped whenever the click lands on the slot.
//
// The save comes FIRST and is verified — the file is written atomically and
// read back, and the game's stacks only move when the disk provably holds
// the new bay; a failed save leaves the cursor's item where it is.
// Copyright (c) 2026 hakergeniusz. MIT.
fn elytra_slot_click(
    game: &mut crate::game::Game<'_>,
    right: bool,
) -> core::ops::ControlFlow<()> {
    // The bay's text state as a real stack, and back — the only fns here
    // that touch the game's item types.
    fn from_text(s: &elytra_slot::SlotStack) -> horizon_client::items::ItemStack {
        let mut stack = horizon_client::items::ItemStack::new(s.item.as_str(), s.count);
        stack.damage = s.damage;
        if !s.ench.is_empty() || s.repair > 0 {
            let mut data = horizon_items::item_data::StackData::default();
            data.enchantments = s
                .ench
                .iter()
                .map(|(id, level)| (std::sync::Arc::from(id.as_str()), *level))
                .collect();
            data.repair_cost = s.repair;
            stack.extra = Some(Box::new(data));
        }
        stack
    }
    fn into_text(stack: &horizon_client::items::ItemStack) -> elytra_slot::SlotStack {
        let (ench, repair) = stack
            .extra
            .as_ref()
            .map_or((Vec::new(), 0), |data| {
                (
                    data.enchantments
                        .iter()
                        .map(|(id, level)| (id.to_string(), *level))
                        .collect(),
                    data.repair_cost,
                )
            });
        elytra_slot::SlotStack {
            item: stack.item.to_string(),
            count: stack.count,
            damage: stack.damage,
            ench,
            repair,
        }
    }

    let _ = right;
    if game.remote.is_some() {
        return core::ops::ControlFlow::Continue(());
    }
    if !elytra_slot::setting_on(elytra_slot::file_setting) {
        return core::ops::ControlFlow::Continue(());
    }
    if !elytra_slot::plain_inventory(
        game.container.is_none(),
        game.station.is_none(),
        game.craft_size == 2,
        game.mount_active(),
        game.creative_tab_open(),
    ) {
        return core::ops::ControlFlow::Continue(());
    }
    let Some(size) = game
        .renderer
        .as_ref()
        .map(|r| [r.config.width, r.config.height])
    else {
        return core::ops::ControlFlow::Continue(());
    };
    let batch = horizon_client::hud::HudBatch::new(size, game.settings.gui_scale_inputs());
    let origin = game.inventory_origin(&batch);
    if !elytra_slot::over_slot(game.cursor, origin) {
        return core::ops::ControlFlow::Continue(());
    }

    // The click is the slot's from here on: nothing vanilla runs after us.
    let cursor_item = game.carried.as_ref().map(|s| s.item.to_string());
    let held = elytra_slot::held();
    let Some(action) = elytra_slot::click_action(cursor_item.as_deref(), held.as_ref()) else {
        // The slot is ours but answers nothing (a non-elytra on the cursor):
        // the click ends here, as it would on any empty place of the panel.
        return core::ops::ControlFlow::Break(());
    };
    match action {
        elytra_slot::Action::Store => {
            let Some(cursor_stack) = game.carried.as_ref() else {
                return core::ops::ControlFlow::Break(());
            };
            let incoming = into_text(cursor_stack);
            // The bay's old stack (if any) takes the cursor's place: a swap,
            // never a loss — but only once the file provably holds the bay.
            if elytra_slot::save_bay(Some(&incoming)).is_err() {
                return core::ops::ControlFlow::Break(());
            }
            game.carried = held.as_ref().map(|old| from_text(old));
            elytra_slot::set(Some(incoming));
        }
        elytra_slot::Action::Take => {
            let Some(old) = held else {
                return core::ops::ControlFlow::Break(());
            };
            // The file goes empty first: a failed save keeps the elytra in
            // the bay, and the next tick's refill reads it back.
            if elytra_slot::save_bay(None).is_err() {
                return core::ops::ControlFlow::Break(());
            }
            elytra_slot::set(None);
            game.carried = Some(from_text(&old));
        }
    }
    core::ops::ControlFlow::Break(())
}
