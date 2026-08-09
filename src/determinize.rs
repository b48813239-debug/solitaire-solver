//! Couche d'incertitude : échantillonnage de mondes + agrégation HOP/PIMC par vote.

use std::collections::HashMap;

use crate::card::Card;
use crate::moves::Move;
use crate::rng::shuffle;
use crate::solver::{solve_with_large_stack, SolveOutput};
use crate::state::State;
use crate::zobrist::NUM_COLUMNS;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct KnownState {
    pub column_visible: [Vec<Card>; NUM_COLUMNS],
    pub column_facedown_count: [u8; NUM_COLUMNS],
    pub foundations: [u8; 4],
    pub waste_known: Vec<Card>,
    // Cartes du talon pas encore tirées LORS DE CE PASSAGE, mais déjà vues lors d'un passage
    // précédent (recyclage) — leur identité et leur ordre de tirage sont donc parfaitement
    // déterminés, PAS une inconnue à échantillonner. Distinct de `unknown_pool` précisément pour
    // ça : les mélanger aurait fait tirer un talon aléatoire fictif là où le vrai talon restant
    // est déjà entièrement connu (cas courant dès qu'un passage complet a eu lieu).
    pub stock_known: Vec<Card>,
    // Cartes du talon jamais vues (uniquement possible avant la fin du tout premier passage) —
    // celles-ci, et seulement celles-ci, sont piochées dans `unknown_pool` au même titre que les
    // cartes de colonnes encore cachées.
    pub undrawn_unknown_count: u8,
    pub unknown_pool: Vec<Card>,
}

impl KnownState {
    fn total_unknown_slots(&self) -> usize {
        self.column_facedown_count.iter().map(|&c| c as usize).sum::<usize>() + self.undrawn_unknown_count as usize
    }
}

pub fn sample_world(known: &KnownState, seed: u64) -> State {
    assert_eq!(
        known.total_unknown_slots(),
        known.unknown_pool.len(),
        "incohérence: {} emplacements inconnus mais {} cartes dans le réservoir",
        known.total_unknown_slots(),
        known.unknown_pool.len()
    );

    let mut pool = known.unknown_pool.clone();
    shuffle(&mut pool, seed);
    let mut idx = 0usize;

    let mut column_cards: [Vec<Card>; NUM_COLUMNS] = std::array::from_fn(|_| Vec::new());
    for col in 0..NUM_COLUMNS {
        let fd = known.column_facedown_count[col] as usize;
        column_cards[col].extend_from_slice(&pool[idx..idx + fd]);
        idx += fd;
        column_cards[col].extend_from_slice(&known.column_visible[col]);
    }

    // Ordre du talon reconstitué : d'abord ce qui a déjà été tiré CE passage (waste_known, sera
    // remis dans le talon puis re-tiré juste en dessous pour atterrir exactement dans le talon
    // où il était), puis ce qui est encore à tirer mais déjà connu (stock_known, dans son ordre
    // réel), puis enfin les vraies inconnues nouvellement échantillonnées. Cet ordre précis
    // importe : c'est lui qui détermine dans quel ordre `state.draw()` les ressort.
    let mut stock = known.waste_known.clone();
    stock.extend_from_slice(&known.stock_known);
    stock.extend_from_slice(&pool[idx..idx + known.undrawn_unknown_count as usize]);
    idx += known.undrawn_unknown_count as usize;
    debug_assert_eq!(idx, pool.len());

    let mut state = State::deal(&column_cards, &known.column_facedown_count, &stock);
    for suit in 0..4 {
        if known.foundations[suit] > 0 {
            state.set_foundation(suit, known.foundations[suit]);
        }
    }
    for _ in 0..known.waste_known.len() {
        state.draw();
    }
    state
}

#[derive(serde::Serialize)]
pub struct WorldOutcome {
    pub solved: bool,
    pub first_move: Option<Move>,
    pub best_foundation_count: u32,
    pub nodes_explored: u64,
}

impl From<SolveOutput> for WorldOutcome {
    fn from(out: SolveOutput) -> Self {
        WorldOutcome {
            solved: out.solved,
            first_move: out.moves.first().copied(),
            best_foundation_count: out.best_foundation_count,
            nodes_explored: out.nodes_explored,
        }
    }
}

pub fn solve_one_world(known: &KnownState, world_seed: u64, budget: u64) -> WorldOutcome {
    let state = sample_world(known, world_seed);
    solve_with_large_stack(state, budget).into()
}

pub struct HopResult {
    pub worlds_run: u32,
    pub worlds_solved: u32,
    pub vote_counts: Vec<(Move, u32)>,
    pub avg_foundation_when_unsolved: f64,
}

pub fn hop_analyze(known: &KnownState, num_worlds: u32, budget_per_world: u64, seed: u64) -> HopResult {
    let mut votes: HashMap<Move, u32> = HashMap::new();
    let mut worlds_solved = 0u32;
    let mut unsolved_progress_sum = 0u64;
    let mut unsolved_count = 0u32;

    for w in 0..num_worlds {
        let world_seed = seed ^ (w as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let outcome = solve_one_world(known, world_seed, budget_per_world);
        if outcome.solved {
            worlds_solved += 1;
            if let Some(mv) = outcome.first_move {
                *votes.entry(mv).or_insert(0) += 1;
            }
        } else {
            unsolved_count += 1;
            unsolved_progress_sum += outcome.best_foundation_count as u64;
        }
    }

    let mut vote_counts: Vec<(Move, u32)> = votes.into_iter().collect();
    vote_counts.sort_by_key(|(_, c)| std::cmp::Reverse(*c));

    HopResult {
        worlds_run: num_worlds,
        worlds_solved,
        vote_counts,
        avg_foundation_when_unsolved: if unsolved_count > 0 {
            unsolved_progress_sum as f64 / unsolved_count as f64
        } else {
            0.0
        },
    }
}
