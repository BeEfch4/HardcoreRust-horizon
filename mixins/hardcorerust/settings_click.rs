fn hardcorerust_settings_click(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    button: usize,
) {
    crate::game::hardcorerust::on_native_settings_click(game, mod_id, button);
}
