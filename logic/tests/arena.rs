use exact_beat_em_up_logic::{
    arena::{CombatPhase, CombatState, Enemy},
    ExactBeatEmUp, Options, ARENA_MODE,
};
use exact_game::Sim;

fn start() -> Sim<ExactBeatEmUp> {
    exact_game::sim_with_art!(Options {
        seed: 17,
        mode: ARENA_MODE,
        ..Options::default()
    })
    .unwrap()
}

#[test]
fn arena_raises_pistols_and_advances_rounds() {
    let mut game = start();
    game.run(500.0);
    let state = game.world().resource::<CombatState>();
    assert!(matches!(
        state.phase,
        CombatPhase::Firing | CombatPhase::RoundClear
    ));
    assert!(state.round >= 1);
    assert!(game.world().query::<&Enemy>().iter().count() <= state.remaining as usize);
}

#[test]
fn arena_player_takes_three_contact_hits() {
    let mut game = start();
    game.world_mut()
        .require_mut::<exact_game::Transform>("player")
        .position = exact_game::Vec3::new(0.0, 0.9, 0.0);
    game.run(4_000.0);
    let state = game.world().resource::<CombatState>();
    assert!(state.player_hits <= 3);
}
