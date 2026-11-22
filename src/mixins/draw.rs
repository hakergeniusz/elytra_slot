// The elytra slot, drawn over the plain survival inventory after the screen
// has drawn its own slots: the slot frame right of the player model, the
// bay's elytra in it — or the dimmed ghost when the bay is empty, the way
// the armor ghosts draw — the vanilla hover highlight and a tooltip naming
// the elytra and the bay's rules. Read-only: the click answers in its own
// helpers. The hook is the screen draw's own frame post, which fires while
// any screen but Playing is up, so the screen here is read off the game.
// Copyright (c) 2026 hakergeniusz. MIT.
fn elytra_slot_draw(game: &crate::game::Game<'_>, batch: &mut HudBatch) {
    // The bay's text state as a real stack — for the draw and the tooltip.
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

    if game.screen != crate::game::Screen::Inventory {
        return;
    }
    if !elytra_slot::setting_on(elytra_slot::file_setting) {
        return;
    }
    if !elytra_slot::plain_inventory(
        game.container.is_none(),
        game.station.is_none(),
        game.craft_size == 2,
        game.mount_active(),
        game.creative_tab_open(),
    ) {
        return;
    }
    let origin = game.inventory_origin(batch);
    let (x, y) = (origin.0 + elytra_slot::SLOT_X, origin.1 + elytra_slot::SLOT_Y);
    let hovered = horizon_client::ui::hovered(
        game.cursor,
        x,
        y,
        elytra_slot::SLOT_SIZE,
        elytra_slot::SLOT_SIZE,
    );
    batch.sprite(&game.gui.atlas, "gui/sprites/container/slot", x, y);
    // While the cursor carries a stack over the slot, the carried stack is
    // already drawn there: the slot's own content stays hidden under it.
    let show_content = !(hovered && game.carried.is_some());
    let held = elytra_slot::held();
    if show_content {
        if let Some(stack) = held.as_ref().map(from_text) {
            game.draw_item(batch, &stack, x, y);
        } else {
            let ghost = horizon_client::items::ItemStack::new(elytra_slot::ELYTRA, 1);
            game.draw_item(batch, &ghost, x, y);
            batch.fill(&game.gui.atlas, x + 1.0, y + 1.0, 16.0, 16.0, 0x9000_0000);
        }
    }
    if hovered {
        horizon_client::ui::blit_sprite(
            batch,
            &game.gui.atlas,
            "gui/sprites/container/slot_highlight_front",
            x - 4.0,
            y - 4.0,
            24.0,
            24.0,
            u32::MAX,
        );
        if game.carried.is_none() {
            let held_stack = held.as_ref().map(from_text);
            let name = held_stack.as_ref().map_or_else(
                || "Elytra Slot".to_owned(),
                |stack| horizon_client::stations::hover_name(&game.pack.names, stack),
            );
            let hint = if game.remote.is_some() {
                "Glides in single player and LAN"
            } else {
                "Your chestplate stays worn beside it"
            };
            game.native_text(batch, &name, [game.cursor.0, game.cursor.1], 0xFF_FFFF, true);
            game.native_text(
                batch,
                hint,
                [game.cursor.0, game.cursor.1 + 10.0],
                0xA0_A0A0,
                true,
            );
        }
    }
}
