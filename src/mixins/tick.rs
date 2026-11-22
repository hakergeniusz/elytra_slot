// Every tick: the bay refills from its config file once it is empty — the
// file is the durable copy (the bay survives restarts and deaths, because
// the elytra it holds is not part of the world's inventory). A missing or
// junk file answers None and clears nothing: the live bay is never taken
// away by a file that cannot be read.
// Copyright (c) 2026 hakergeniusz. MIT.
fn elytra_slot_tick(game: &mut crate::game::Game<'_>) {
    let _ = game;
    if elytra_slot::held().is_none() && elytra_slot::setting_on(elytra_slot::file_setting) {
        if let Some(stack) = elytra_slot::load_bay() {
            elytra_slot::set(Some(stack));
        }
    }
}
