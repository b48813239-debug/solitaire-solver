//! Construction d'un `State` à partir d'un jeu de 52 cartes mélangé.

use crate::card::{all_cards, Card, NUM_CARDS};
use crate::rng::shuffle;
use crate::state::State;
use crate::zobrist::NUM_COLUMNS;

pub fn deal_from_shuffle(shuffled: &[Card; NUM_CARDS]) -> State {
    let mut column_cards: [Vec<Card>; NUM_COLUMNS] = std::array::from_fn(|_| Vec::new());
    let mut column_facedown = [0u8; NUM_COLUMNS];
    let mut idx = 0usize;
    for col in 0..NUM_COLUMNS {
        let size = col + 1;
        for i in 0..size {
            column_cards[col].push(shuffled[idx]);
            idx += 1;
            if i < size - 1 {
                column_facedown[col] += 1;
            }
        }
    }
    let stock: Vec<Card> = shuffled[idx..].to_vec();
    debug_assert_eq!(stock.len(), NUM_CARDS - idx);
    State::deal(&column_cards, &column_facedown, &stock)
}

pub fn shuffled_deck(seed: u64) -> [Card; NUM_CARDS] {
    let mut deck = all_cards();
    shuffle(&mut deck, seed);
    deck
}

pub fn random_deal(seed: u64) -> State {
    deal_from_shuffle(&shuffled_deck(seed))
}
