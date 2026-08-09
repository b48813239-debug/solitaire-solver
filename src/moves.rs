//! Génération des coups légaux à partir d'un `State`.
//!
//! `MoveList` est un tampon à taille fixe (pas de `Vec`) : la génération de
//! coups a lieu à chaque nœud exploré par le solveur, potentiellement des
//! millions de fois par résolution — une allocation tas par nœud serait le
//! premier goulot d'étranglement.

use crate::card::{opposite_color, rank_value, suit_of, Card};
use crate::state::{Column, State};
use crate::zobrist::NUM_COLUMNS;

pub const MAX_MOVES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Move {
    Draw,
    Recycle,
    WasteToFoundation,
    WasteToColumn { dest: u8 },
    ColumnToFoundation { from: u8 },
    ColumnToColumn { from: u8, dest: u8, count: u8 },
}

impl Default for Move {
    fn default() -> Self {
        Move::Draw
    }
}

pub struct MoveList {
    pub items: [Move; MAX_MOVES],
    pub len: usize,
}

impl MoveList {
    pub fn new() -> Self {
        MoveList { items: [Move::default(); MAX_MOVES], len: 0 }
    }

    #[inline(always)]
    fn push(&mut self, m: Move) {
        debug_assert!(self.len < MAX_MOVES, "MAX_MOVES dépassé — augmenter la capacité");
        if self.len < MAX_MOVES {
            self.items[self.len] = m;
            self.len += 1;
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[Move] {
        &self.items[..self.len]
    }
}

#[inline]
fn can_place_on_tableau(card: Card, dest: &Column) -> bool {
    if dest.is_empty() {
        rank_value(card) == 13
    } else {
        match dest.top() {
            Some(top) => opposite_color(card, top) && rank_value(card) + 1 == rank_value(top),
            None => false,
        }
    }
}

/// Index physique le plus bas à partir duquel la suite jusqu'au sommet forme
/// un enchaînement valide (couleurs alternées, rangs strictement décroissants
/// de 1 en montant). Tout index dans `[résultat, col.len-1]` est un départ de
/// déplacement valide.
#[inline]
fn valid_run_start_min(col: &Column) -> u8 {
    if col.face_up_count() == 0 {
        return col.len;
    }
    let mut i = col.len - 1;
    while i > col.face_down {
        let a = col.cards[(i - 1) as usize];
        let b = col.cards[i as usize];
        if rank_value(a) == rank_value(b) + 1 && opposite_color(a, b) {
            i -= 1;
        } else {
            break;
        }
    }
    i
}

pub fn gen_moves(state: &State, moves: &mut MoveList) {
    moves.len = 0;

    if state.stock_pointer < state.stock_len {
        moves.push(Move::Draw);
    } else if state.stock_pointer == state.stock_len && state.stock_len > 0 {
        moves.push(Move::Recycle);
    }

    if let Some(card) = state.stock_top() {
        let suit = suit_of(card);
        if state.foundations[suit] + 1 == rank_value(card) {
            moves.push(Move::WasteToFoundation);
        }
        for dest in 0..NUM_COLUMNS {
            if can_place_on_tableau(card, &state.columns[dest]) {
                moves.push(Move::WasteToColumn { dest: dest as u8 });
            }
        }
    }

    for from in 0..NUM_COLUMNS {
        let col = &state.columns[from];

        if let Some(top) = col.top() {
            let suit = suit_of(top);
            if state.foundations[suit] + 1 == rank_value(top) {
                moves.push(Move::ColumnToFoundation { from: from as u8 });
            }
        }

        if col.face_up_count() > 0 {
            let min_start = valid_run_start_min(col);
            let mut start = col.len - 1;
            loop {
                let count = col.len - start;
                let bottom_of_run = col.cards[start as usize];
                for dest in 0..NUM_COLUMNS {
                    if dest == from {
                        continue;
                    }
                    if can_place_on_tableau(bottom_of_run, &state.columns[dest]) {
                        moves.push(Move::ColumnToColumn { from: from as u8, dest: dest as u8, count });
                    }
                }
                if start == min_start {
                    break;
                }
                start -= 1;
            }
        }
    }
}
