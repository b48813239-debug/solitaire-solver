//! Recherche de solution sur un monde entièrement déterminisé.

use std::collections::HashSet;

use crate::apply::{make_move, unmake_move};
use crate::card::{is_red, rank_value, suit_of, Card};
use crate::moves::{gen_moves, Move, MoveList};
use crate::state::State;
use crate::zobrist::NUM_COLUMNS;

const RED_FOUNDATIONS: [usize; 2] = [crate::card::COEUR, crate::card::CARREAU];
const BLACK_FOUNDATIONS: [usize; 2] = [crate::card::PIQUE, crate::card::TREFLE];

#[inline]
fn is_safe_to_autoplay(state: &State, card: Card) -> bool {
    let r = rank_value(card);
    if r <= 2 {
        return true;
    }
    let opp = if is_red(card) { BLACK_FOUNDATIONS } else { RED_FOUNDATIONS };
    let min_opp = state.foundations[opp[0]].min(state.foundations[opp[1]]);
    min_opp >= r - 1
}

fn find_safe_autoplay(state: &State) -> Option<Move> {
    if let Some(card) = state.stock_top() {
        let suit = suit_of(card);
        if state.foundations[suit] + 1 == rank_value(card) && is_safe_to_autoplay(state, card) {
            return Some(Move::WasteToFoundation);
        }
    }
    for from in 0..NUM_COLUMNS {
        if let Some(card) = state.columns[from].top() {
            let suit = suit_of(card);
            if state.foundations[suit] + 1 == rank_value(card) && is_safe_to_autoplay(state, card) {
                return Some(Move::ColumnToFoundation { from: from as u8 });
            }
        }
    }
    None
}

fn move_priority(state: &State, mv: &Move) -> i32 {
    match *mv {
        Move::ColumnToFoundation { .. } | Move::WasteToFoundation => 1000,
        Move::ColumnToColumn { from, count, .. } => {
            let col = &state.columns[from as usize];
            let reveals = count == col.face_up_count() && col.face_down > 0;
            if reveals {
                500
            } else {
                100
            }
        }
        Move::WasteToColumn { .. } => 200,
        Move::Draw => 10,
        Move::Recycle => 5,
    }
}

enum Outcome {
    Won,
    Exhausted,
    BudgetReached,
}

struct Ctx {
    visited: HashSet<u64>,
    nodes: u64,
    budget: u64,
    best_foundation_count: u32,
}

fn dfs(state: &mut State, path: &mut Vec<Move>, ctx: &mut Ctx) -> Outcome {
    if state.is_won() {
        return Outcome::Won;
    }
    if ctx.nodes >= ctx.budget {
        return Outcome::BudgetReached;
    }
    if !ctx.visited.insert(state.hash) {
        return Outcome::Exhausted;
    }
    let fc = state.foundation_count();
    if fc > ctx.best_foundation_count {
        ctx.best_foundation_count = fc;
    }

    if let Some(mv) = find_safe_autoplay(state) {
        ctx.nodes += 1;
        let undo = make_move(state, mv);
        path.push(mv);
        let outcome = dfs(state, path, ctx);
        if !matches!(outcome, Outcome::Won) {
            path.pop();
            unmake_move(state, &undo);
        }
        return outcome;
    }

    let mut moves = MoveList::new();
    gen_moves(state, &mut moves);
    let mut ordered: Vec<Move> = moves.as_slice().to_vec();
    ordered.sort_by_key(|m| std::cmp::Reverse(move_priority(state, m)));

    for mv in ordered {
        if ctx.nodes >= ctx.budget {
            return Outcome::BudgetReached;
        }
        ctx.nodes += 1;
        let undo = make_move(state, mv);
        path.push(mv);
        match dfs(state, path, ctx) {
            Outcome::Won => return Outcome::Won,
            Outcome::BudgetReached => {
                path.pop();
                unmake_move(state, &undo);
                return Outcome::BudgetReached;
            }
            Outcome::Exhausted => {
                path.pop();
                unmake_move(state, &undo);
            }
        }
    }
    Outcome::Exhausted
}

pub struct SolveOutput {
    pub solved: bool,
    pub moves: Vec<Move>,
    pub nodes_explored: u64,
    pub budget_reached: bool,
    pub best_foundation_count: u32,
}

pub fn solve(state: State, budget: u64) -> SolveOutput {
    solve_with_large_stack(state, budget)
}

fn solve_on_current_thread(mut state: State, budget: u64) -> SolveOutput {
    let mut ctx = Ctx { visited: HashSet::new(), nodes: 0, budget, best_foundation_count: state.foundation_count() };
    let mut path = Vec::new();
    match dfs(&mut state, &mut path, &mut ctx) {
        Outcome::Won => SolveOutput {
            solved: true,
            moves: path,
            nodes_explored: ctx.nodes,
            budget_reached: false,
            best_foundation_count: 52,
        },
        Outcome::Exhausted => SolveOutput {
            solved: false,
            moves: Vec::new(),
            nodes_explored: ctx.nodes,
            budget_reached: false,
            best_foundation_count: ctx.best_foundation_count,
        },
        Outcome::BudgetReached => SolveOutput {
            solved: false,
            moves: Vec::new(),
            nodes_explored: ctx.nodes,
            budget_reached: true,
            best_foundation_count: ctx.best_foundation_count,
        },
    }
}

pub fn solve_with_large_stack(state: State, budget: u64) -> SolveOutput {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || solve_on_current_thread(state, budget))
        .expect("échec de création du thread de résolution")
        .join()
        .expect("le thread de résolution a paniqué")
}
