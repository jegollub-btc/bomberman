//! Phase 5: power-ups are collected.

use crate::game::{Event, GameState, PowerupKind};

pub fn run(game: &mut GameState, events: &mut Vec<Event>) {
    let mut claims: Vec<(usize, usize)> = Vec::new();
    for (player_index, player) in game.players.iter().enumerate() {
        if !player.alive {
            continue;
        }
        if let Some(powerup_index) = game.powerups.iter().position(|p| p.cell == player.cell) {
            claims.push((player_index, powerup_index));
        }
    }

    // Remove from the back so the earlier indices stay valid.
    claims.sort_unstable_by_key(|&(_, powerup_index)| std::cmp::Reverse(powerup_index));

    for (player_index, powerup_index) in claims {
        let powerup = game.powerups.remove(powerup_index);
        let rules = game.rules;
        let player = &mut game.players[player_index];
        match powerup.kind {
            PowerupKind::ExtraBomb => player.bombs_max = player.bombs_max.saturating_add(1),
            PowerupKind::Flame => player.flame = (player.flame + 1).min(rules.max_flame),
            PowerupKind::Speed => player.speed = (player.speed + 1).min(rules.max_speed),
        }
        events.push(Event::PowerupRemoved {
            id: powerup.id,
            cell: powerup.cell,
            kind: powerup.kind,
            taken_by: Some(player.id),
        });
        events.push(player.stats_event());
    }
}
