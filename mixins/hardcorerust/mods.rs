fn hardcorerust_mods(
    game: &mut crate::game::Game<'_>,
    mods: &mut Vec<horizon_client::mod_api::NativeMod>,
) {
    crate::game::hardcorerust::on_native_mods(game, mods);
}
