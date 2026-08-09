//! Encodage d'une carte sur un seul octet : `suit * 13 + rank0`.
//! - `rank0` va de 0 (As) à 12 (Roi).
//! - `suit`  va de 0 à 3 : 0=Cœur, 1=Carreau, 2=Pique, 3=Trèfle
//!   (cœur/carreau = rouge, pique/trèfle = noir).
//!
//! Choisi pour tenir dans un `u8`, permettre une indexation directe dans les
//! tables de Zobrist (`carte -> u64`) et éviter toute allocation : c'est la
//! représentation la plus dense possible pour 52 valeurs distinctes.

pub type Card = u8;

/// Sentinelle "aucune carte" (emplacement vide dans un tableau à taille fixe).
pub const NONE: Card = 255;

pub const NUM_SUITS: usize = 4;
pub const NUM_RANKS: usize = 13;
pub const NUM_CARDS: usize = NUM_SUITS * NUM_RANKS;

pub const COEUR: usize = 0;
pub const CARREAU: usize = 1;
pub const PIQUE: usize = 2;
pub const TREFLE: usize = 3;

#[inline(always)]
pub fn make_card(suit: usize, rank0: u8) -> Card {
    debug_assert!(suit < NUM_SUITS && (rank0 as usize) < NUM_RANKS);
    (suit as u8) * (NUM_RANKS as u8) + rank0
}

#[inline(always)]
pub fn suit_of(c: Card) -> usize {
    (c / NUM_RANKS as u8) as usize
}

/// Rang 0-indexé : 0=As .. 12=Roi.
#[inline(always)]
pub fn rank0_of(c: Card) -> u8 {
    c % NUM_RANKS as u8
}

/// Valeur usuelle du rang : 1=As .. 13=Roi (utile pour comparer aux fondations).
#[inline(always)]
pub fn rank_value(c: Card) -> u8 {
    rank0_of(c) + 1
}

#[inline(always)]
pub fn is_red(c: Card) -> bool {
    matches!(suit_of(c), COEUR | CARREAU)
}

#[inline(always)]
pub fn opposite_color(a: Card, b: Card) -> bool {
    is_red(a) != is_red(b)
}

/// Liste des 52 cartes dans l'ordre canonique (utile pour construire un monde déterminisé).
pub fn all_cards() -> [Card; NUM_CARDS] {
    let mut out = [0u8; NUM_CARDS];
    let mut i = 0usize;
    for suit in 0..NUM_SUITS {
        for rank0 in 0..NUM_RANKS {
            out[i] = make_card(suit, rank0 as u8);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for suit in 0..NUM_SUITS {
            for rank0 in 0..NUM_RANKS {
                let c = make_card(suit, rank0 as u8);
                assert_eq!(suit_of(c), suit);
                assert_eq!(rank0_of(c), rank0 as u8);
            }
        }
    }

    #[test]
    fn colors() {
        assert!(is_red(make_card(COEUR, 0)));
        assert!(is_red(make_card(CARREAU, 5)));
        assert!(!is_red(make_card(PIQUE, 5)));
        assert!(!is_red(make_card(TREFLE, 12)));
    }
}
