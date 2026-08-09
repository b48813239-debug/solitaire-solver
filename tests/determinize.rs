use solitaire_solver::card::{all_cards, make_card, Card};
use solitaire_solver::{hop_analyze, sample_world, solve_one_world, KnownState};

fn near_solved_known_state() -> KnownState {
    let mut column_visible: [Vec<Card>; 7] = std::array::from_fn(|_| Vec::new());
    column_visible[0] = vec![make_card(3, 0)];
    column_visible[6] = (4..=12u8).rev().map(|rank0| make_card(3, rank0)).collect();

    let column_facedown_count = [1u8, 0, 0, 0, 0, 0, 0];
    let foundations = [13u8, 13, 13, 0];

    let unknown_pool = vec![make_card(3, 1), make_card(3, 2), make_card(3, 3)];

    KnownState {
        column_visible,
        column_facedown_count,
        foundations,
        waste_known: vec![],
        undrawn_count: 2,
        unknown_pool,
    }
}

#[test]
fn sample_world_respects_known_projection() {
    let known = near_solved_known_state();
    for seed in 0..10u64 {
        let world = sample_world(&known, seed * 31 + 1);

        let mut count = 0u32;
        for col in &world.columns {
            count += col.len as u32;
        }
        count += world.stock_len as u32;
        count += world.foundations.iter().map(|&f| f as u32).sum::<u32>();
        assert_eq!(count, all_cards().len() as u32, "un monde tiré doit toujours totaliser 52 cartes");

        assert_eq!(world.columns[0].top().unwrap(), make_card(3, 0));
        assert_eq!(world.columns[6].top().unwrap(), make_card(3, 4));
        assert_eq!(world.foundations, [13, 13, 13, 0]);
    }
}

#[test]
fn solve_one_world_wins_every_sampled_world() {
    let known = near_solved_known_state();
    for seed in 0..15u64 {
        let out = solve_one_world(&known, seed * 17 + 5, 50_000);
        assert!(out.solved, "monde seed={seed} non résolu alors que la position est triviale");
    }
}

#[test]
fn hop_analyze_produces_a_consistent_vote() {
    let known = near_solved_known_state();
    let result = hop_analyze(&known, 20, 50_000, 12345);
    assert_eq!(result.worlds_solved, 20, "tous les mondes de ce test sont censés être gagnables");
    assert!(!result.vote_counts.is_empty());
    let total_votes: u32 = result.vote_counts.iter().map(|(_, c)| c).sum();
    assert_eq!(total_votes, 20);
}
