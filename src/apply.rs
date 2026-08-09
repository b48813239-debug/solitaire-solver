//! `make_move` / `unmake_move` — le cœur de la vitesse du solveur.

use crate::card::{suit_of, Card};
use crate::moves::Move;
use crate::state::State;

pub enum UndoKind {
    Draw,
    Recycle { old_pointer: u8 },
    WasteToFoundation { suit: usize, old_level: u8, card: Card, old_pointer: u8, old_stock_len: u8 },
    WasteToColumn { dest: usize, card: Card, old_pointer: u8, old_stock_len: u8 },
    ColumnToFoundation { from: usize, suit: usize, old_level: u8, card: Card, revealed: bool },
    ColumnToColumn { from: usize, dest: usize, count: u8, revealed: bool },
}

pub struct Undo {
    pub kind: UndoKind,
}

pub fn make_move(state: &mut State, mv: Move) -> Undo {
    let kind = match mv {
        Move::Draw => {
            state.draw();
            UndoKind::Draw
        }
        Move::Recycle => {
            let old_pointer = state.stock_pointer;
            state.recycle();
            UndoKind::Recycle { old_pointer }
        }
        Move::WasteToFoundation => {
            let old_pointer = state.stock_pointer;
            let old_stock_len = state.stock_len;
            let card = state.stock_remove_top();
            let suit = suit_of(card);
            let old_level = state.foundations[suit];
            state.set_foundation(suit, old_level + 1);
            UndoKind::WasteToFoundation { suit, old_level, card, old_pointer, old_stock_len }
        }
        Move::WasteToColumn { dest } => {
            let old_pointer = state.stock_pointer;
            let old_stock_len = state.stock_len;
            let card = state.stock_remove_top();
            state.col_push(dest as usize, card);
            UndoKind::WasteToColumn { dest: dest as usize, card, old_pointer, old_stock_len }
        }
        Move::ColumnToFoundation { from } => {
            let card = state.columns[from as usize].top().expect("coup illégal: colonne vide");
            let suit = suit_of(card);
            let old_level = state.foundations[suit];
            state.col_pop(from as usize);
            let revealed = state.maybe_reveal(from as usize);
            state.set_foundation(suit, old_level + 1);
            UndoKind::ColumnToFoundation { from: from as usize, suit, old_level, card, revealed }
        }
        Move::ColumnToColumn { from, dest, count } => {
            let mut buf = [0u8; 24];
            for i in 0..count {
                buf[i as usize] = state.col_pop(from as usize);
            }
            let revealed = state.maybe_reveal(from as usize);
            for i in (0..count).rev() {
                state.col_push(dest as usize, buf[i as usize]);
            }
            UndoKind::ColumnToColumn { from: from as usize, dest: dest as usize, count, revealed }
        }
    };
    Undo { kind }
}

pub fn unmake_move(state: &mut State, undo: &Undo) {
    match undo.kind {
        UndoKind::Draw => state.undraw(),
        UndoKind::Recycle { old_pointer } => state.unrecycle(old_pointer),
        UndoKind::WasteToFoundation { suit, old_level, card, old_pointer, old_stock_len } => {
            state.set_foundation(suit, old_level);
            state.stock_reinsert(card, old_pointer, old_stock_len);
        }
        UndoKind::WasteToColumn { dest, card, old_pointer, old_stock_len } => {
            state.col_pop(dest);
            state.stock_reinsert(card, old_pointer, old_stock_len);
        }
        UndoKind::ColumnToFoundation { from, suit, old_level, card, revealed } => {
            state.set_foundation(suit, old_level);
            if revealed {
                state.unreveal(from);
            }
            state.col_push(from, card);
        }
        UndoKind::ColumnToColumn { from, dest, count, revealed } => {
            let mut buf = [0u8; 24];
            for i in 0..count {
                buf[i as usize] = state.col_pop(dest);
            }
            if revealed {
                state.unreveal(from);
            }
            for i in (0..count).rev() {
                state.col_push(from, buf[i as usize]);
            }
        }
    }
}
