//! Phase 8: is the match over?
//!
//! Two ways to end: one player left standing, or the round timer expiring.

use crate::game::{scoring, EndReason, Event, GameState, MatchOutcome, PlayerResult};
use crate::shared::PlayerId;

pub fn run(game: &mut GameState, events: &mut Vec<Event>) -> Option<MatchOutcome> {
    let alive = game.alive_count();
    let timed_out = game.tick >= game.rules.round_time_ticks;

    // A normal match ends with one player left. A solo match -- practice, a
    // test, a single bot warming up -- would end on tick one under that rule,
    // so it runs until that player dies instead.
    let survivors_needed = if game.players.len() > 1 { 2 } else { 1 };
    if alive >= survivors_needed && !timed_out {
        return None;
    }

    let reason = if alive < survivors_needed {
        EndReason::LastStanding
    } else {
        EndReason::Timeout
    };

    for player in game.players.iter_mut().filter(|p| p.alive) {
        player.score = player.score.saturating_add(scoring::SCORE_SURVIVAL);
    }
    for index in 0..game.players.len() {
        events.push(game.players[index].stats_event());
    }

    game.finished = true;
    Some(outcome(game, reason))
}

/// Rank players: survivors first, then whoever lasted longest, then by score.
pub fn outcome(game: &GameState, reason: EndReason) -> MatchOutcome {
    let mut ranked: Vec<&crate::game::Player> = game.players.iter().collect();
    ranked.sort_by_key(|p| {
        (
            !p.alive,
            std::cmp::Reverse(p.died_at.unwrap_or(u32::MAX)),
            std::cmp::Reverse(p.score),
        )
    });

    let mut results = Vec::with_capacity(ranked.len());
    let mut placement = 0u8;
    let mut previous: Option<(bool, u32, u16)> = None;
    for (position, player) in ranked.iter().enumerate() {
        let key = (player.alive, player.died_at.unwrap_or(u32::MAX), player.score);
        // Tied players share a placement and the next distinct player skips
        // ahead, so the table reads like a leaderboard rather than a list.
        if previous != Some(key) {
            placement = position as u8 + 1;
            previous = Some(key);
        }
        results.push(PlayerResult {
            id: player.id,
            placement,
            score: player.score,
        });
    }
    results.sort_by_key(|r| r.id);

    MatchOutcome {
        reason,
        winner: winner(game),
        results,
    }
}

fn winner(game: &GameState) -> Option<PlayerId> {
    let survivors: Vec<&crate::game::Player> =
        game.players.iter().filter(|p| p.alive).collect();
    match survivors.len() {
        0 => None,
        1 => Some(survivors[0].id),
        // Timed out with several alive: highest score takes it, a tie is a draw.
        _ => {
            let best = survivors.iter().map(|p| p.score).max().unwrap_or(0);
            let leaders: Vec<&&crate::game::Player> =
                survivors.iter().filter(|p| p.score == best).collect();
            (leaders.len() == 1).then(|| leaders[0].id)
        }
    }
}
