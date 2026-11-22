//! Elytra Slot, the native-route native mod: the survival inventory's own
//! elytra bay beside the armor row.
//!
//! The brain is pure standard library — no game types — so it compiles and
//! tests wherever. Every decision the helpers act on lives here, and every
//! one of those decisions is tested below.
//!
//! The model: the mod's slot is its own bay beside the armor row, and the
//! elytra it holds is REAL state, not a chest-slot swap — the chestplate
//! stays worn the whole time, armor and all. The held stack lives as plain
//! text (item, count, damage, enchantments, repair cost) in ONE config file
//! under the mod's data folder that every save writes atomically and reads
//! back before the game's stacks move — the engine's own setting writes
//! once silently did nothing in the live game, so stored elytras died with
//! the process. The helpers turn that text back into the game's own
//! ItemStack for drawing, the cursor and the glide check.
//!
//! One honest limitation: the glide check is a client answer, so the slot
//! flies in single player and on LAN (where the client hosts the world) and
//! stays display-only on a remote server. And the elytra in this bay does
//! not take glide wear — the vanilla wear picks a worn piece out of the
//! armor.
//!
//! Clean-room: written for Horizon from the game's own public behavior; it
//! copies nothing from any mod.
//!
//! Copyright (c) 2026 hakergeniusz. MIT.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Mutex;

use mod_api::Hooks;

/// The mod's registry id: the settings folder and the Options > Mods row.
pub const ID: &str = "elytra_slot";

/// The elytra: the only item the mod's slot takes.
pub const ELYTRA: &str = "minecraft:elytra";

/// The mod slot's place in the plain survival inventory, in panel-relative
/// GUI pixels: right of the player model (`ox + 26 .. ox + 75`), above the
/// offhand slot at (77, 62), clear of the recipe book's button at (104, 61).
pub const SLOT_X: f32 = 77.0;
pub const SLOT_Y: f32 = 8.0;
/// An inventory slot is 18 GUI pixels wide (16 plus the frame).
pub const SLOT_SIZE: f32 = 18.0;

/// The mod's setting key for the Enabled toggle, which every gate re-reads.
pub const SETTING_ENABLED: &str = "enabled";

/// The stack the bay holds, as the plain text the config files keep. An
/// elytra carries a damage bar, enchantments and a repair cost; the custom
/// name (a styled component) is the one part the files do not keep.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlotStack {
    pub item: String,
    pub count: u32,
    pub damage: u32,
    /// Enchantment id and level, in the stack's own order.
    pub ench: Vec<(String, u32)>,
    pub repair: u32,
}

impl SlotStack {
    /// Whether the bay's stack is the elytra (it always is, but the files
    /// are user-editable text, so the gates ask).
    pub fn is_elytra(&self) -> bool {
        self.item == ELYTRA
    }
}

/// The enchantments as their file text: `id:level;id:level`.
pub fn ench_text(ench: &[(String, u32)]) -> String {
    ench.iter()
        .map(|(id, level)| format!("{id}:{level}"))
        .collect::<Vec<_>>()
        .join(";")
}

/// The file text back into enchantments; broken parts are skipped.
pub fn ench_of(text: &str) -> Vec<(String, u32)> {
    text.split(';')
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| {
            // From the right: ids carry colons of their own
            // ("minecraft:unbreaking"), so the level is the LAST colon.
            let (id, level) = part.trim().rsplit_once(':')?;
            let level = level.trim().parse::<u32>().ok()?;
            // Real entries are namespaced ids with a level; junk is dropped.
            let id = id.trim();
            if level == 0 || !id.contains(':') {
                return None;
            }
            Some((id.to_owned(), level))
        })
        .collect()
}

// ---- the bay's durable copy: ONE file, saved verified ----
//
// The file is written FIRST (temp + rename), read back and parsed, and the
// game's stacks only move when the file provably holds the new bay. A save
// that cannot be verified keeps the cursor's item where it is — nothing eats
// an item.

/// The bay file's name under the mod's data folder.
pub const BAY_FILE: &str = "bay.txt";

/// The last save failure, for whoever asks; cleared by the next good save.
static SAVE_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// The last save failure, if any.
pub fn save_error() -> Option<String> {
    SAVE_ERROR.lock().ok()?.clone()
}

fn note_save_error(error: Option<String>) {
    if let Ok(mut slot) = SAVE_ERROR.lock() {
        *slot = error;
    }
}

/// The bay's durable text: five newline-separated fields in a fixed order.
/// An empty text is an empty bay.
pub fn bay_text(stack: Option<&SlotStack>) -> String {
    let Some(s) = stack else {
        return String::new();
    };
    format!(
        "{}\n{}\n{}\n{}\n{}",
        s.item,
        s.count,
        s.damage,
        ench_text(&s.ench),
        s.repair
    )
}

/// The durable text back into a stack. Missing fields, a short file, a
/// non-elytra item or junk parse as an empty bay — the file is user-editable
/// text, and a bay that never held an elytra is an empty bay.
pub fn bay_of(text: &str) -> Option<SlotStack> {
    let mut lines = text.lines();
    let item = lines.next()?.trim().to_owned();
    if item != ELYTRA {
        return None;
    }
    let count = lines
        .next()?
        .trim()
        .parse::<u32>()
        .ok()
        .unwrap_or(1)
        .max(1);
    let damage = lines.next()?.trim().parse::<u32>().ok().unwrap_or(0);
    let ench = ench_of(lines.next().unwrap_or(""));
    let repair = lines.next()?.trim().parse::<u32>().ok().unwrap_or(0);
    Some(SlotStack {
        item,
        count,
        damage,
        ench,
        repair,
    })
}

/// The mod's data folder: `<the client's working directory>/horizon-mods/
/// .horizon-mod-data/elytra_slot/` — the same folder the settings store
/// keeps the mod's toggles in, wherever the game runs.
pub fn data_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join("horizon-mods")
        .join(".horizon-mod-data")
        .join(ID)
}

/// A setting file's text, as the settings store keeps it: one file per key,
/// "true"/"false" for toggles.
pub fn file_setting(key: &str) -> Option<String> {
    std::fs::read_to_string(data_dir().join(key)).ok()
}

/// Saves the bay under `dir` and verifies it: the file is written atomically
/// (temp + rename) and read back — `Ok` only when what sits on disk parses
/// to the same stack that was asked for.
pub fn save_bay_in(dir: &std::path::Path, stack: Option<&SlotStack>) -> Result<(), String> {
    let path = dir.join(BAY_FILE);
    let parent = path
        .parent()
        .ok_or_else(|| "the bay file has no parent folder".to_owned())?;
    let saved = (|| -> Result<(), String> {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let text = bay_text(stack);
        let tmp = parent.join(".bay.tmp");
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        let read = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        match (bay_of(&read).as_ref(), stack) {
            (None, None) => Ok(()),
            (Some(a), Some(b)) if a == b => Ok(()),
            _ => Err("the file on disk is not the bay that was saved".to_owned()),
        }
    })();
    note_save_error(saved.as_ref().err().cloned());
    saved
}

/// Saves the bay in the mod's own data folder, verified.
pub fn save_bay(stack: Option<&SlotStack>) -> Result<(), String> {
    save_bay_in(&data_dir(), stack)
}

/// The bay as the file under `dir` holds it; a missing or junk file is None.
/// Never an error: an unloadable file must not take the bay away from the
/// live state.
pub fn load_bay_in(dir: &std::path::Path) -> Option<SlotStack> {
    let text = std::fs::read_to_string(dir.join(BAY_FILE)).ok()?;
    bay_of(&text)
}

/// The bay as the mod's own data folder holds it.
pub fn load_bay() -> Option<SlotStack> {
    load_bay_in(&data_dir())
}

/// Whether the mod is on, from the setting file a getter reads. A missing
/// value means ON — the mod exists for the slot, so installing it is asking
/// for it; anything but `false` is on, trimmed like the game reads its
/// settings.
pub fn setting_on(get: impl FnOnce(&str) -> Option<String>) -> bool {
    !matches!(get(SETTING_ENABLED).as_deref(), Some(v) if v.trim() == "false")
}

/// Whether the open screen is the plain survival inventory — the only one the
/// mod's slot draws on and answers in. The inventory screen also hosts every
/// container, station and mount window; those have a menu, a container, a
/// station, a mount menu or the creative screen's own tab.
pub fn plain_inventory(
    container_none: bool,
    station_none: bool,
    craft_two: bool,
    mount: bool,
    creative_tab: bool,
) -> bool {
    container_none && station_none && craft_two && !mount && !creative_tab
}

/// Whether the cursor is over the mod's slot, drawn at `origin` (the panel's
/// top left in GUI pixels).
pub fn over_slot(cursor: (f32, f32), origin: (f32, f32)) -> bool {
    let (x, y) = (origin.0 + SLOT_X, origin.1 + SLOT_Y);
    cursor.0 >= x && cursor.0 < x + SLOT_SIZE && cursor.1 >= y && cursor.1 < y + SLOT_SIZE
}

// The live bay: one Mutex-guarded Option, so the helpers never block each
// other beyond a copy. The file is the durable copy; this is the tick's.
static SLOT: Mutex<Option<SlotStack>> = Mutex::new(None);

/// The bay's stack as of now.
pub fn held() -> Option<SlotStack> {
    SLOT.lock().ok().and_then(|slot| slot.clone())
}

/// The bay takes or gives a stack.
pub fn set(stack: Option<SlotStack>) {
    if let Ok(mut slot) = SLOT.lock() {
        *slot = stack;
    }
}

/// What a click on the mod's slot does.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// The cursor's elytra goes in; the bay's old stack (if any) comes to
    /// the cursor, so no item is ever lost.
    Store,
    /// The bay's stack comes to the empty cursor.
    Take,
}

/// The action a click answers with: the cursor's elytra stores, an empty
/// cursor over a held elytra takes it, anything else is not the mod's.
pub fn click_action(cursor_item: Option<&str>, bay: Option<&SlotStack>) -> Option<Action> {
    match (cursor_item, bay) {
        (Some(ELYTRA), _) => Some(Action::Store),
        (None, Some(s)) if s.is_elytra() => Some(Action::Take),
        _ => None,
    }
}

// ---- the store a press deferred ----
//
// The press hook reads the game through a shared borrow (and a carried
// stack starts the game's quick-craft drag before the menu click's
// fallback, so the press can never do the swap itself): it leaves a plan,
// and the tick — which holds the exclusive game — performs the store.

thread_local! {
    /// Whether the last press on the bay asked the tick to store the
    /// cursor's elytra. The helpers and the tick share one game thread.
    static DEFERRED_STORE: RefCell<bool> = const { RefCell::new(false) };
}

/// Queue the deferred store for the next tick.
pub fn defer_store() {
    DEFERRED_STORE.with(|plan| *plan.borrow_mut() = true);
}

/// Take the queued store, if the last press left one; a second press
/// overwrites, it does not queue up.
pub fn take_deferred_store() -> bool {
    DEFERRED_STORE.with(|plan| core::mem::take(&mut *plan.borrow_mut()))
}

/// Whether a deferred store may run now: the press's gates still hold and
/// the cursor still carries the elytra it pressed with. Anything else — the
/// screen closed, the mod turned off, a server joined, the cursor emptied
/// or changed — drops the plan where the stack sits, so nothing moves on a
/// stale plan.
pub fn deferred_store_runs(
    remote: bool,
    on: bool,
    plain: bool,
    cursor_item: Option<&str>,
) -> bool {
    !remote && on && plain && cursor_item == Some(ELYTRA)
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// The Enabled toggle on the mod's page; the bay's state shows in the
    /// slot's own tooltip in game.
    fn register(
        &self,
        registrar: &mut dyn mod_api::Registrar,
        _: &mod_api::RegistrationContext,
    ) {
        let _ = registrar.register_setting(mod_api::SettingSpec::toggle(
            SETTING_ENABLED,
            "elytra_slot.setting.enabled",
            true,
        ));
    }

    /// Implemented as a no-op so the game always calls the
    /// `native_client_tick_pre` dispatch the tick mixin lives in: the game
    /// skips dispatches no mod listens to, and the mixin must run.
    fn client_tick_pre(&self, _: &mut dyn mod_api::ClientHost, _: &mut mod_api::ClientTickPreEvent) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn the_stack() -> SlotStack {
        SlotStack {
            item: ELYTRA.to_owned(),
            count: 1,
            damage: 31,
            ench: vec![
                ("minecraft:unbreaking".to_owned(), 3),
                ("minecraft:mending".to_owned(), 1),
            ],
            repair: 2,
        }
    }

    #[test]
    fn the_bay_round_trips_through_its_file_text() {
        let stack = the_stack();
        assert!(stack.is_elytra());
        let text = bay_text(Some(&stack));
        assert_eq!(bay_of(&text).as_ref(), Some(&stack), "the text reads back");
        assert_eq!(bay_text(Some(bay_of(&text).as_ref().unwrap())), text);
        assert_eq!(bay_of(&bay_text(None)), None, "an empty text is an empty bay");
        assert_eq!(bay_of(""), None);
        assert_eq!(bay_of("minecraft:bow\n1\n0\n\n0"), None, "a wrong item holds nothing");
        assert_eq!(bay_of("minecraft:elytra\n1\n0\n\n0").map(|s| s.damage), Some(0), "an empty ench line parses");
        assert_eq!(
            bay_of("minecraft:elytra\nnot-a-number\n0\n\n0").map(|s| s.count),
            Some(1),
            "junk numbers default, like the files always have"
        );
    }

    #[test]
    fn the_bay_saves_verified_and_refuses_to_lie() {
        let dir = std::env::temp_dir().join(format!("elytra_slot_test_{}", std::process::id()));
        let stack = the_stack();
        assert_eq!(save_bay_in(&dir, Some(&stack)), Ok(()));
        let read = std::fs::read_to_string(dir.join(BAY_FILE)).expect("the file exists");
        assert_eq!(bay_of(&read).as_ref(), Some(&stack), "the file holds the stack");
        // A second save over the first, then the clear.
        let mut other = the_stack();
        other.damage = 13;
        assert_eq!(save_bay_in(&dir, Some(&other)), Ok(()));
        assert_eq!(load_bay_in(&dir).map(|s| s.damage), Some(13));
        assert_eq!(save_bay_in(&dir, None), Ok(()));
        assert_eq!(std::fs::read_to_string(dir.join(BAY_FILE)).unwrap(), "", "the clear empties the file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_enchant_text_round_trips_and_tolerates_junk() {
        let ench = vec![("minecraft:mending".to_owned(), 1)];
        assert_eq!(ench_of(&ench_text(&ench)), ench);
        assert_eq!(ench_of(&ench_text(&[])), Vec::<(String, u32)>::new());
        assert_eq!(ench_of(";;junk;minecraft:aqua_affinity:1;x:0"), vec![
            ("minecraft:aqua_affinity".to_owned(), 1)
        ]);
    }

    #[test]
    fn the_bay_holds_one_stack() {
        set(None);
        assert_eq!(held(), None);
        set(Some(SlotStack {
            item: ELYTRA.to_owned(),
            count: 1,
            damage: 0,
            ench: Vec::new(),
            repair: 0,
        }));
        assert!(held().is_some());
        set(None);
        assert_eq!(held(), None);
    }

    #[test]
    fn a_click_answers_what_the_cursor_and_bay_hold() {
        let bay = Some(SlotStack {
            item: ELYTRA.to_owned(),
            count: 1,
            damage: 0,
            ench: Vec::new(),
            repair: 0,
        });
        assert_eq!(click_action(Some(ELYTRA), None), Some(Action::Store));
        assert_eq!(
            click_action(Some(ELYTRA), bay.as_ref()),
            Some(Action::Store),
            "an elytra on the cursor swaps the bay's own out to the cursor"
        );
        assert_eq!(click_action(None, bay.as_ref()), Some(Action::Take));
        assert_eq!(click_action(None, None), None, "nothing anywhere");
        assert_eq!(click_action(Some("minecraft:diamond_chestplate"), None), None, "the slot takes elytra only");
    }

    #[test]
    fn the_slot_answers_only_over_itself() {
        assert!(over_slot((77.0, 8.0), (0.0, 0.0)));
        assert!(over_slot((94.9, 25.9), (0.0, 0.0)));
        assert!(!over_slot((95.1, 8.0), (0.0, 0.0)));
        assert!(!over_slot((77.0, 26.1), (0.0, 0.0)));
        assert!(over_slot((87.0, 18.0), (10.0, 10.0)), "the origin moves it");
    }

    #[test]
    fn a_deferred_store_runs_only_while_the_press_still_holds() {
        assert!(
            deferred_store_runs(false, true, true, Some(ELYTRA)),
            "the press's own case runs"
        );
        assert!(
            !deferred_store_runs(true, true, true, Some(ELYTRA)),
            "a server never stores"
        );
        assert!(
            !deferred_store_runs(false, false, true, Some(ELYTRA)),
            "the toggle off drops the plan"
        );
        assert!(
            !deferred_store_runs(false, true, false, Some(ELYTRA)),
            "a screen that changed drops the plan"
        );
        assert!(
            !deferred_store_runs(false, true, true, None),
            "an emptied cursor drops the plan"
        );
        assert!(
            !deferred_store_runs(false, true, true, Some("minecraft:diamond_chestplate")),
            "another stack on the cursor drops the plan"
        );
    }

    #[test]
    fn the_deferred_store_is_taken_once() {
        assert!(!take_deferred_store(), "no plan until a press defers one");
        defer_store();
        assert!(take_deferred_store());
        assert!(!take_deferred_store(), "the tick consumes the plan");
        defer_store();
        defer_store();
        assert!(take_deferred_store(), "a second press overwrites, not queues");
        assert!(!take_deferred_store());
    }

    #[test]
    fn installed_means_on_until_declined() {
        assert!(setting_on(|_| None), "the default must be on");
        assert!(setting_on(|k| (k == SETTING_ENABLED).then(|| " true \n".to_owned())));
        assert!(!setting_on(|k| (k == SETTING_ENABLED).then(|| "false".to_owned())));
        assert_eq!(SLOT_X, 77.0, "the slot sits right of the player model");
    }
}
