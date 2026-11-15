fn hardcorerust_craft(
    game: &mut crate::game::Game<'_>,
    result: &mut Option<horizon_client::items::ItemStack>,
) {
    crate::game::hardcorerust::on_prepare_craft(game, result);
}
