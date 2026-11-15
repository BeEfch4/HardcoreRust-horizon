use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

pub struct HardcoreState {
    pub initialized: bool,
    pub impossible_mode: Option<bool>,
    pub mob_knowledge: u32,
    pub bleeding_seconds: u32,
    pub panic_ticks: u32,
    pub last_hit_tick: i64,
    pub last_player_health: f32,
    pub last_food_level: i32,
    pub bandage_cooldown: u32,
    pub known_mobs: HashMap<u32, f32>,
    pub combat_witnesses: HashSet<u32>,
    pub shared_knowledge_mobs: HashSet<u32>,
    pub equipped_mobs: HashSet<u32>,
    pub mob_armor_cache: HashMap<u32, Vec<String>>,
    pub checked_chests: HashSet<(i32, i32, i32)>,
}

pub fn state() -> &'static Mutex<HardcoreState> {
    static STATE: OnceLock<Mutex<HardcoreState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(HardcoreState {
            initialized: false,
            impossible_mode: None,
            mob_knowledge: 0,
            bleeding_seconds: 0,
            panic_ticks: 0,
            last_hit_tick: -100,
            last_player_health: 20.0,
            last_food_level: 20,
            bandage_cooldown: 0,
            known_mobs: HashMap::new(),
            combat_witnesses: HashSet::new(),
            shared_knowledge_mobs: HashSet::new(),
            equipped_mobs: HashSet::new(),
            mob_armor_cache: HashMap::new(),
            checked_chests: HashSet::new(),
        })
    })
}

fn is_impossible_enabled(game: &crate::game::Game<'_>) -> bool {
    let mut st = state().lock().unwrap();
    if let Some(enabled) = st.impossible_mode {
        return enabled;
    }
    let saved = game
        .native_setting("hardcorerust", "impossible")
        .is_some_and(|v| v == "true");
    st.impossible_mode = Some(saved);
    saved
}

pub fn on_native_mods(
    _game: &mut crate::game::Game<'_>,
    mods: &mut Vec<horizon_client::mod_api::NativeMod>,
) {
    mods.push(horizon_client::mod_api::NativeMod {
        id: "hardcorerust".to_string(),
        title: "HardcoreRust 1.0".to_string(),
        description: "Реалистичное выживание и отдельная сложность «Невозможно».".to_string(),
        keys: Vec::new(),
    });
}

pub fn on_native_settings(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    if mod_id != "hardcorerust" {
        return;
    }
    let enabled = is_impossible_enabled(game);
    let label = if enabled {
        "Сложность: Невозможно [ВКЛ]"
    } else {
        "Сложность: Невозможно [ВЫКЛ]"
    };
    buttons.push(label.to_string());
}

pub fn on_native_settings_click(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    button: usize,
) {
    if mod_id != "hardcorerust" || button != 0 {
        return;
    }
    let next = !is_impossible_enabled(game);
    {
        let mut st = state().lock().unwrap();
        st.impossible_mode = Some(next);
    }
    game.native_set_setting("hardcorerust", "impossible", if next { "true" } else { "false" });
}

fn create_ushanka_item() -> horizon_client::items::ItemStack {
    let mut hat = horizon_client::items::ItemStack::new("minecraft:leather_helmet", 1);
    let mut data = horizon_client::item_data::StackData::default();
    data.set_other(
        "minecraft:custom_name",
        Some(horizon_world::nbt::Tag::String("§fШапка-ушанка".into())),
    );
    data.set_other(
        "minecraft:dyed_color",
        Some(horizon_world::nbt::Tag::Int(0x1A1A1D)),
    );
    hat.extra = Some(Box::new(data));
    hat
}

fn is_ushanka(stack: &horizon_client::items::ItemStack) -> bool {
    if &*stack.item != "minecraft:leather_helmet" {
        return false;
    }
    stack
        .custom_name()
        .is_some_and(|n| n.to_lowercase().contains("ушанка"))
}

pub fn on_prepare_craft(
    game: &mut crate::game::Game<'_>,
    result: &mut Option<horizon_client::items::ItemStack>,
) {
    if !is_impossible_enabled(game) {
        return;
    }
    if let Some(res) = result.as_ref() {
        if &*res.item == "minecraft:leather_helmet" {
            let has_black_dye = res
                .extra
                .as_ref()
                .is_some_and(|e| e.other.iter().any(|(k, _)| k == "minecraft:dyed_color"));
            if has_black_dye {
                *result = Some(create_ushanka_item());
            }
        }
    }
}

fn is_light_item(id: &str) -> bool {
    matches!(
        id,
        "minecraft:torch"
            | "minecraft:soul_torch"
            | "minecraft:lantern"
            | "minecraft:soul_lantern"
            | "minecraft:jack_o_lantern"
            | "minecraft:lava_bucket"
    )
}

fn is_heat_block(name: &str) -> bool {
    matches!(
        name,
        "minecraft:torch"
            | "minecraft:wall_torch"
            | "minecraft:soul_torch"
            | "minecraft:soul_wall_torch"
            | "minecraft:lantern"
            | "minecraft:soul_lantern"
            | "minecraft:campfire"
            | "minecraft:soul_campfire"
            | "minecraft:furnace"
            | "minecraft:blast_furnace"
            | "minecraft:smoker"
            | "minecraft:fire"
            | "minecraft:soul_fire"
            | "minecraft:lava"
            | "minecraft:glowstone"
            | "minecraft:jack_o_lantern"
    )
}

fn is_hostile_mob(id: &str) -> bool {
    let clean = id.trim_start_matches("minecraft:");
    matches!(
        clean,
        "zombie"
            | "zombie_villager"
            | "husk"
            | "drowned"
            | "skeleton"
            | "stray"
            | "bogged"
            | "creeper"
            | "spider"
            | "cave_spider"
            | "enderman"
            | "witch"
            | "pillager"
            | "vindicator"
            | "piglin"
            | "piglin_brute"
            | "zombified_piglin"
    )
}

fn burns_in_sunlight(clean_name: &str) -> bool {
    matches!(
        clean_name,
        "zombie" | "zombie_villager" | "drowned" | "skeleton" | "stray" | "bogged"
    )
}

fn has_warmth_or_light(game: &crate::game::Game<'_>, px: i32, py: i32, pz: i32) -> bool {
    if let Some(main_hand) = game.inventory.get(game.selected).and_then(|s| s.as_ref()) {
        if is_light_item(&main_hand.item) {
            return true;
        }
    }
    if let Some(off_hand) = game.offhand.as_ref() {
        if is_light_item(&off_hand.item) {
            return true;
        }
    }
    let facts = game.res.facts();
    for dx in -4..=4 {
        for dy in -2..=2 {
            for dz in -4..=4 {
                let block_id = game.world.block(px + dx, py + dy, pz + dz);
                if is_heat_block(&facts.get(block_id).name) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn on_game_tick(game: &mut crate::game::Game<'_>) {
    if game.world.game_time < 40 || game.creative || game.spectator() {
        return;
    }

    if !is_impossible_enabled(game) {
        let mut st = state().lock().unwrap();
        if st.bleeding_seconds > 0 || st.panic_ticks > 0 {
            st.bleeding_seconds = 0;
            st.panic_ticks = 0;
            game.potions
                .effects
                .list
                .retain(|e| e.id != "nausea" && e.id != "slowness" && e.id != "wither");
            game.apply_effect_status();
        }
        return;
    }

    let now_tick = game.world.game_time;
    let time_of_day = game.world.day_time.rem_euclid(24000);
    let days_passed = (game.world.day_time.max(0) / 24000) as u32;
    let is_night = time_of_day > 13000 && time_of_day < 23000;
    let is_morning = time_of_day >= 22500 || time_of_day <= 1500;

    let wearing_ushanka = game
        .armor
        .first()
        .and_then(|s| s.as_ref())
        .is_some_and(is_ushanka);

    let (px_f, py_f, pz_f, sprinting, current_hp, current_food) = {
        let Some(player) = game.player.as_ref() else {
            return;
        };
        (
            player.position.x,
            player.position.y,
            player.position.z,
            player.sprinting,
            player.health.health(),
            player.health.food.food,
        )
    };
    let (px, py, pz) = (px_f.floor() as i32, py_f.floor() as i32, pz_f.floor() as i32);

    // === ПУНКТ 4: СПАВН БИНТОВ В СУНДУКАХ ДЕРЕВЕНЬ, АВАНПОСТОВ И ПОСЕЛЕНИЙ ===
    if let Some(chest_pos) = game.container {
        let already_checked = {
            let mut st = state().lock().unwrap();
            !st.checked_chests.insert(chest_pos)
        };
        if !already_checked {
            if let Some(horizon_client::containers::BlockEntity::Chest(slots)) =
                game.world.block_entities.get_mut(&chest_pos)
            {
                let has_existing_loot = slots.iter().any(|s| s.is_some());
                let rand_val = ((now_tick as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(chest_pos.0 as u64)
                    % 100) as u32;

                if has_existing_loot && rand_val < 45 {
                    if let Some(empty_slot) = slots.iter_mut().find(|s| s.is_none()) {
                        let count = (rand_val % 3) + 1;
                        *empty_slot = Some(horizon_client::items::ItemStack::new(
                            "minecraft:paper",
                            count,
                        ));
                    }
                }
            }
        }
    }

    let mut apply_night_bonus_damage = false;
    let mut night_bonus_amp: i32 = 0;
    let mut used_bandage_now = false;
    let mut armor_drops_to_spawn: Vec<(horizon_client::items::ItemStack, [f64; 3])> = Vec::new();

    {
        let mut st = state().lock().unwrap();
        if !st.initialized {
            st.initialized = true;
            st.last_player_health = current_hp;
            st.last_food_level = current_food;
            return;
        }

        if st.bandage_cooldown > 0 {
            st.bandage_cooldown -= 1;
        }

        if current_hp <= 0.0 {
st.bleeding_seconds = 0;
st.panic_ticks = 0;
st.last_player_health = 0.0;
return;
}
// === ПУНКТ 3: КРОВОТЕЧЕНИЕ ПРИ СИЛЬНОМ УДАРЕ (> 8 СЕРДЕЦ = > 16.0 УРОНА) ===
if current_hp < st.last_player_health {
let dmg_taken = st.last_player_health - current_hp;
st.last_hit_tick = now_tick;
if dmg_taken > 16.0 {
let rand_sec = ((now_tick as u64)
.wrapping_mul(1103515245)
.wrapping_add(12345)
% 29) as u32
+ 2;
st.bleeding_seconds = rand_sec;
}
if is_night && days_passed > 0 && dmg_taken >= 1.5 {
apply_night_bonus_damage = true;
night_bonus_amp = ((days_passed / 5).min(2)) as i32;
}
}
st.last_player_health = current_hp;
// Остановка кровотечения едой
if current_food > st.last_food_level && st.bleeding_seconds > 0 {
st.bleeding_seconds = 0;
}
st.last_food_level = current_food;
// Остановка кровотечения бинтом (ПКМ с бумагой в руке)
if st.bleeding_seconds > 0 && game.right && st.bandage_cooldown == 0 {
let main_is_bandage = game
.inventory
.get(game.selected)
.and_then(|s| s.as_ref())
.is_some_and(|s| &*s.item == "minecraft:paper");
let off_is_bandage = game
.offhand
.as_ref()
.is_some_and(|s| &*s.item == "minecraft:paper");
if main_is_bandage || off_is_bandage {
st.bleeding_seconds = 0;
st.bandage_cooldown = 15;
used_bandage_now = true;
}
}
// === ПУНКТЫ 2, 5, 6 И 7: ТАКТИКА МОБОВ, ШАПКИ, БРОНЯ И УШАНКА ===
let speed_progress = if is_night {
(days_passed as f64 * 0.1).min(1.0)
} else {
0.0
};
let night_speed_mult = 1.0 + speed_progress;
let player_attacking = game.attack_ticks < 5;
for mob in game.mobs.iter_mut() {
if mob.removed || !mob.alive() {
if let Some(worn_items) = st.mob_armor_cache.remove(&mob.id) {
if game.last_hurt_by_player.contains_key(&mob.id) {
for (idx, item_id) in worn_items.into_iter().enumerate() {
let drop_roll = ((now_tick as u64)
.wrapping_mul(2862933555777941757)
.wrapping_add(mob.id as u64 + idx as u64)
% 100) as u32;
if drop_roll < 2 {
let stack = if item_id == "ushanka" {
create_ushanka_item()
} else {
horizon_client::items::ItemStack::new(&item_id, 1)
};
armor_drops_to_spawn.push((
stack,
[mob.position.x, mob.position.y + 0.5, mob.position.z],
));
}
}
}
}
st.known_mobs.remove(&mob.id);
st.combat_witnesses.remove(&mob.id);
st.shared_knowledge_mobs.remove(&mob.id);
st.equipped_mobs.remove(&mob.id);
continue;
}
let mob_id_str = mob.kind.id.as_str();
if !is_hostile_mob(mob_id_str) {
continue;
}
let clean_name = mob_id_str.trim_start_matches("minecraft:");
if !st.equipped_mobs.contains(&mob.id) {
st.equipped_mobs.insert(mob.id);
let mut worn_list: Vec<String> = Vec::new();
let seed = (now_tick as u64)
.wrapping_mul(1103515245)
.wrapping_add((mob.id as u64).wrapping_mul(9973));
let seed2 = seed
.wrapping_mul(6364136223846793005)
.wrapping_add(1442695040888963407);
let ultra_rare_roll = ((seed as u128) << 64) | (seed2 as u128);
if ultra_rare_roll % 100_000_000_000_000_000 == 777 {
if let Some(head_slot) = mob.extra.armor.first_mut() {
*head_slot = Some(create_ushanka_item());
}
worn_list.push("ushanka".to_string());
} else {
let armor_roll = (seed % 100) as u32;
let armor_chance = if matches!(
clean_name,
"zombie"
| "zombie_villager"
| "husk"
| "drowned"
| "skeleton"
| "stray"
| "bogged"
) {
5
} else {
2
};
if armor_roll < armor_chance {
let tier = ((seed / 100) % 3) as u32;
let set: [&str; 4] = if matches!(
clean_name,
"piglin" | "piglin_brute" | "zombified_piglin"
) {
[
"minecraft:netherite_helmet",
"minecraft:netherite_chestplate",
"minecraft:netherite_leggings",
"minecraft:netherite_boots",
]
} else {
match tier {
0 => [
"minecraft:leather_helmet",
"minecraft:leather_chestplate",
"minecraft:leather_leggings",
"minecraft:leather_boots",
],
1 => [
"minecraft:chainmail_helmet",
"minecraft:chainmail_chestplate",
"minecraft:chainmail_leggings",
"minecraft:chainmail_boots",
],
_ => [
"minecraft:iron_helmet",
"minecraft:iron_chestplate",
"minecraft:iron_leggings",
"minecraft:iron_boots",
],
}
};
for slot in 0..4 {
let piece_roll = ((seed >> (slot * 4)) % 10) as u32;
if piece_roll < 7 {
if let Some(armor_slot) = mob.extra.armor.get_mut(slot) {
*armor_slot =
Some(horizon_client::items::ItemStack::new(set[slot], 1));
}
worn_list.push(set[slot].to_string());
}
}
}
if burns_in_sunlight(clean_name)
&& st.mob_knowledge > 0
&& mob.extra.armor.first().is_none_or(|s| s.is_none())
{
let hat_chance = (st.mob_knowledge * 15).min(75);
let hat_roll = ((seed >> 16) % 100) as u32;
if hat_roll < hat_chance {
if let Some(head_slot) = mob.extra.armor.first_mut() {
*head_slot = Some(horizon_client::items::ItemStack::new(
"minecraft:leather_helmet",
1,
));
}
worn_list.push("minecraft:leather_helmet".to_string());
}
}
}
if !worn_list.is_empty() {
st.mob_armor_cache.insert(mob.id, worn_list);
}
}
let dx = mob.position.x - px_f;
let dy = mob.position.y - py_f;
let dz = mob.position.z - pz_f;
let dist_sq = dx * dx + dy * dy + dz * dz;
let dist = dist_sq.sqrt();
let prev_mob_hp = *st.known_mobs.get(&mob.id).unwrap_or(&mob.health);
if dist < 18.0
&& (player_attacking
|| mob.health < prev_mob_hp
|| (now_tick - st.last_hit_tick) < 10)
{
st.combat_witnesses.insert(mob.id);
}
if speed_progress > 0.0 && dist < 32.0 {
let tick_boost = 1.0 + (speed_progress * 0.29);
mob.velocity.x *= tick_boost;
mob.velocity.z *= tick_boost;
}
let dodge_chance = if is_night {
(0.08 + days_passed as f64 * 0.03).min(0.45)
} else {
0.05
};
if dist < 3.8 && (player_attacking || mob.health < prev_mob_hp) {
let pseudo_rand = ((now_tick as u64)
.wrapping_mul(1103515245)
.wrapping_add(mob.id as u64)
% 100) as f64
/ 100.0;
if pseudo_rand < dodge_chance {
let dir = if (mob.id + now_tick as u32) % 2 == 0 {
1.0
} else {
-1.0
};
let perp_x = -dz / dist.max(0.1) * 0.38 * dir;
let perp_z = dx / dist.max(0.1) * 0.38 * dir;
mob.velocity.x += perp_x;
mob.velocity.z += perp_z;
}
}
st.known_mobs.insert(mob.id, mob.health);
if matches!(clean_name, "skeleton" | "stray" | "bogged") && dist < 5.5 && dist > 0.1 {
let away_x = (dx / dist) * 0.22 * night_speed_mult;
let away_z = (dz / dist) * 0.22 * night_speed_mult;
mob.velocity.x = away_x;
mob.velocity.z = away_z;
}
if is_morning && st.combat_witnesses.contains(&mob.id) {
let mx = mob.position.x.floor() as i32;
let my = mob.position.y.floor() as i32;
let mz = mob.position.z.floor() as i32;
let sky_light = game.world.light(mx, my, mz) >> 4;
if sky_light >= 8 {
let mut best_dir = (0.0, -0.15, 0.0);
let mut lowest_sky = sky_light;
for (ox, oz) in [(2, 0), (-2, 0), (0, 2), (0, -2)] {
let s_light = game.world.light(mx + ox, my, mz + oz) >> 4;
if s_light < lowest_sky {
lowest_sky = s_light;
best_dir = (ox as f64 * 0.16, -0.1, oz as f64 * 0.16);
}
}
mob.velocity.x = best_dir.0;
mob.velocity.y += best_dir.1;
mob.velocity.z = best_dir.2;
} else if !st.shared_knowledge_mobs.contains(&mob.id) {
st.shared_knowledge_mobs.insert(mob.id);
let chance_50 = ((now_tick as u32).wrapping_add(mob.id)) % 2 == 0;
if chance_50 {
st.mob_knowledge += 1;
}
}
}
}
}
for (stack, pos) in armor_drops_to_spawn {
game.spawn_item(stack, pos, [0.0, 0.2, 0.0], 10);
}
if used_bandage_now {
let mut consumed = false;
if let Some(Some(main_stack)) = game.inventory.get_mut(game.selected) {
if &*main_stack.item == "minecraft:paper" {
if main_stack.count > 1 {
main_stack.count -= 1;
} else {
game.inventory[game.selected] = None;
}
consumed = true;
}
}
if !consumed {
if let Some(off_stack) = game.offhand.as_mut() {
if &*off_stack.item == "minecraft:paper" {
if off_stack.count > 1 {
off_stack.count -= 1;
} else {
game.offhand = None;
}
}
}
}
game.potions.effects.list.retain(|e| e.id != "wither");
game.apply_effect_status();
}
if apply_night_bonus_damage {
let mut extra_dmg =
horizon_client::effects::EffectInstance::new("poison", 25, night_bonus_amp);
extra_dmg.visible = false;
extra_dmg.show_icon = false;
game.add_player_effect(extra_dmg);
}
// === ПРОВЕРКИ РАЗ В СЕКУНДУ ===
if now_tick % 20 != 0 {
return;
}
let is_bleeding = {
let mut st = state().lock().unwrap();
if st.bleeding_seconds > 0 {
st.bleeding_seconds -= 1;
true
} else {
false
}
};
if is_bleeding {
let mut bleed_fx = horizon_client::effects::EffectInstance::new("wither", 25, 0);
bleed_fx.visible = false;
game.add_player_effect(bleed_fx);
}
let light_byte = game.world.light(px, py, pz);
let block_light = light_byte & 15;
let is_dark_or_cold = block_light <= 4 || is_night || py < 50;
let is_warm = has_warmth_or_light(game, px, py, pz);
// Пункт 1: Истощение и усталость
if !is_warm && (is_dark_or_cold || sprinting) {
if sprinting && is_dark_or_cold {
if let Some(p) = game.player.as_mut() {
p.cause_food_exhaustion(if wearing_ushanka { 0.4 } else { 0.8 });
}
let mut hunger = horizon_client::effects::EffectInstance::new(
"hunger",
60,
if wearing_ushanka { 0 } else { 1 },
);
hunger.visible = false;
game.add_player_effect(hunger);
let mut fatigue =
horizon_client::effects::EffectInstance::new("mining_fatigue", 60, 0);
fatigue.visible = false;
game.add_player_effect(fatigue);
} else if is_dark_or_cold && !wearing_ushanka {
if let Some(p) = game.player.as_mut() {
p.cause_food_exhaustion(0.35);
}
let mut hunger = horizon_client::effects::EffectInstance::new("hunger", 60, 0);
hunger.visible = false;
game.add_player_effect(hunger);
let mut fatigue =
horizon_client::effects::EffectInstance::new("mining_fatigue", 60, 0);
fatigue.visible = false;
game.add_player_effect(fatigue);
} else if sprinting {
if let Some(p) = game.player.as_mut() {
p.cause_food_exhaustion(0.3);
}
}
}
// Пункт 2: Паника и холод в полной темноте
let calm_for_2_sec = {
let st = state().lock().unwrap();
(now_tick - st.last_hit_tick) >= 40
};
if !is_warm && block_light == 0 && (is_night || py < 55) {
let do_panic_damage = {
let mut st = state().lock().unwrap();
st.panic_ticks += 1;
st.panic_ticks % 2 == 0
};
let mut nausea_fx = horizon_client::effects::EffectInstance::new("nausea", 100, 0);
nausea_fx.visible = false;
game.add_player_effect(nausea_fx);
if !wearing_ushanka {
let mut slow_fx = horizon_client::effects::EffectInstance::new("slowness", 60, 0);
slow_fx.visible = false;
game.add_player_effect(slow_fx);
}
if do_panic_damage && current_hp > 1.0 && !wearing_ushanka {
let mut cold_dmg = horizon_client::effects::EffectInstance::new("poison", 25, 0);
cold_dmg.visible = false;
cold_dmg.show_icon = false;
game.add_player_effect(cold_dmg);
}
} else if is_warm && calm_for_2_sec {
{
let mut st = state().lock().unwrap();
st.panic_ticks = 0;
}
game.potions
.effects
.list
.retain(|e| e.id != "nausea" && e.id != "slowness" && e.id != "poison");
game.apply_effect_status();
}
}