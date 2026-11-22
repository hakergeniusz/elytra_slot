// A press on the survival inventory that lands on the mod's slot while the
// cursor already carries a stack. The game's own press starts a quick-craft
// drag from a carried stack and returns early, so the click helper never
// sees this press and a carried elytra could never go into the bay — this
// hook, which fires before the screen acts on any press, takes it instead:
// no drag starts, nothing drops. The hook only reads the game, so the store
// itself is deferred to the tick, which re-checks these gates before it
// moves anything — a plan left behind by a closed screen, a turned-off mod
// or an emptied cursor moves nothing.
// Copyright (c) 2026 hakergeniusz. MIT.
fn elytra_slot_press(
    game: &crate::game::Game<'_>,
    screen: &ScreenView<'_>,
) -> core::ops::ControlFlow<()> {
    if screen.id != "inventory" || screen.menu.is_some() {
        return core::ops::ControlFlow::Continue(());
    }
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
    // The press is the slot's only when the cursor carries the elytra: the
    // empty cursor's take answers in the click helper, and any other stack
    // on the cursor stays vanilla's (a drag over the empty place of the
    // panel gathers nothing).
    if !game
        .carried
        .as_ref()
        .is_some_and(|stack| stack.item.to_string() == elytra_slot::ELYTRA)
    {
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
    elytra_slot::defer_store();
    core::ops::ControlFlow::Break(())
}
