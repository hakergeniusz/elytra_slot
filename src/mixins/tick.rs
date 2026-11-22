// Every tick: the bay refills from its config file once it is empty — the
// file is the durable copy (the bay survives restarts and deaths, because
// the elytra it holds is not part of the world's inventory). A missing or
// junk file answers None and clears nothing: the live bay is never taken
// away by a file that cannot be read.
//
// Then the store a press on the slot deferred runs here, where the
// exclusive game is: the press hook only reads the game (a carried stack
// starts the game's quick-craft drag before the click fallback, so the
// swap cannot happen in the press), and the tick repeats the press's gates
// before it moves anything — nothing moves on a stale plan.
// Copyright (c) 2026 hakergeniusz. MIT.
fn elytra_slot_tick(game: &mut crate::game::Game<'_>) {
    // The bay's text state as a real stack, and back — the same moves the
    // click helper makes. Helpers are one fn each and cannot share fns, so
    // the conversions repeat here.
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

    if elytra_slot::held().is_none() && elytra_slot::setting_on(elytra_slot::file_setting) {
        if let Some(stack) = elytra_slot::load_bay() {
            elytra_slot::set(Some(stack));
        }
    }
    if !elytra_slot::take_deferred_store() {
        return;
    }
    // The press's gates, re-checked live: a screen that closed, the mod
    // turned off, a server the client joined, or a cursor that emptied or
    // changed between the press and this tick drops the plan where the
    // stack sits.
    if !elytra_slot::deferred_store_runs(
        game.remote.is_some(),
        elytra_slot::setting_on(elytra_slot::file_setting),
        elytra_slot::plain_inventory(
            game.container.is_none(),
            game.station.is_none(),
            game.craft_size == 2,
            game.mount_active(),
            game.creative_tab_open(),
        ),
        game.carried
            .as_ref()
            .map(|stack| stack.item.to_string())
            .as_deref(),
    ) {
        return;
    }
    // The store, the click helper's own swap: the save comes FIRST and is
    // verified — a failed save keeps the elytra on the cursor, and the next
    // press on the bay tries again. The bay's old stack (if any) takes the
    // cursor's place, so no item is ever lost.
    let Some(cursor_stack) = game.carried.as_ref() else {
        return;
    };
    let incoming = into_text(cursor_stack);
    if elytra_slot::save_bay(Some(&incoming)).is_err() {
        return;
    }
    let old = elytra_slot::held();
    game.carried = old.as_ref().map(|old| from_text(old));
    elytra_slot::set(Some(incoming));
}
