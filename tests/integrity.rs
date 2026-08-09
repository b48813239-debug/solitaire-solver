use solitaire_solver::apply::{make_move, unmake_move};
use solitaire_solver::dealer::random_deal;
use solitaire_solver::moves::{gen_moves, MoveList};
use solitaire_solver::state::State;

fn fingerprint(s: &State) -> String {
    let mut out = String::new();
    for col in &s.columns {
        out.push_str(&format!("{:?}|{}|{};", &col.cards[..col.len as usize], col.len, col.face_down));
    }
    out.push_str(&format!("F{:?};", s.foundations));
    out.push_str(&format!("S{:?}|{}|{};", &s.stock[..s.stock_len as usize], s.stock_len, s.stock_pointer));
    out.push_str(&format!("H{}", s.hash));
    out
}

#[test]
fn make_unmake_roundtrip_is_exact() {
    for seed in 0..30u64 {
        let mut state = random_deal(seed * 7919 + 1);
        let mut rng = seed.wrapping_mul(2654435761).wrapping_add(1);
        for _ in 0..400 {
            let before = fingerprint(&state);
            let mut moves = MoveList::new();
            gen_moves(&state, &mut moves);
            if moves.len == 0 {
                break;
            }
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let mv = moves.as_slice()[(rng as usize) % moves.len];

            let undo = make_move(&mut state, mv);
            unmake_move(&mut state, &undo);
            let after = fingerprint(&state);
            assert_eq!(before, after, "seed={seed}, coup={mv:?} n'a pas été annulé exactement");

            let _ = make_move(&mut state, mv);
        }
    }
}

#[test]
fn hash_matches_from_scratch_recomputation() {
    for seed in 0..20u64 {
        let mut state = random_deal(seed * 104729 + 3);
        let mut rng = seed.wrapping_mul(40503) + 7;
        for _ in 0..200 {
            let mut moves = MoveList::new();
            gen_moves(&state, &mut moves);
            if moves.len == 0 {
                break;
            }
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let mv = moves.as_slice()[(rng as usize) % moves.len];
            make_move(&mut state, mv);

            let incremental = state.hash;
            state.recompute_hash_from_scratch();
            assert_eq!(incremental, state.hash, "seed={seed}: hash incrémental divergent après {mv:?}");
        }
    }
}

#[test]
fn solver_finds_a_trivial_win() {
    use solitaire_solver::card::make_card;
    use solitaire_solver::state::State as S;

    let mut column_cards: [Vec<u8>; 7] = std::array::from_fn(|_| Vec::new());
    for suit in 0..4usize {
        column_cards[suit] = (0..13u8).rev().map(|rank0| make_card(suit, rank0)).collect();
    }
    let facedown = [0u8; 7];
    let state = S::deal(&column_cards, &facedown, &[]);

    let out = solitaire_solver::solve(state, 100_000);
    assert!(out.solved, "position triviale non résolue (nœuds explorés: {})", out.nodes_explored);
    assert_eq!(out.moves.len(), 52);
}

#[test]
fn random_deals_terminate_without_panicking() {
    for seed in 0..8u64 {
        let state = random_deal(seed);
        let out = solitaire_solver::solve(state, 50_000);
        assert!(out.nodes_explored <= 50_000);
        if out.solved {
            assert!(!out.moves.is_empty());
        }
    }
}
