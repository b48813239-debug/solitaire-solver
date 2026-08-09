//! Tables de hachage de Zobrist.
//!
//! Chaque "fait élémentaire" possible sur l'état (une carte à telle position
//! dans telle colonne, telle colonne ayant N cartes cachées, telle fondation
//! à tel niveau, telle carte à telle position dans la pioche, tel pointeur de
//! pioche) reçoit un u64 aléatoire fixé une fois pour toutes au démarrage.
//! Le hash de l'état est le XOR de tous les faits actuellement vrais.
//!
//! Propriété clé : XOR étant sa propre inverse, ajouter/retirer un fait est
//! une simple opération `hash ^= table[...]`, dans les deux sens. C'est ce
//! qui permet au solveur de maintenir le hash de façon incrémentale à chaque
//! coup, sans jamais recalculer l'état entier (contrairement à un hachage par
//! sérialisation, coûteux et alloué à chaque nœud).

use std::sync::OnceLock;

pub const NUM_COLUMNS: usize = 7;
/// Longueur max physique d'une colonne (13 rois + toutes les cartes qu'on
/// pourrait empiler dessus dans le pire des cas ; 24 est très large de marge).
pub const MAX_COL_LEN: usize = 24;
/// Nombre max de cartes qui transitent par la pioche/talon (52 - 28 déjà en
/// colonnes = 24).
pub const MAX_STOCK: usize = 24;

pub struct Zobrist {
    pub col_card: [[[u64; 52]; MAX_COL_LEN]; NUM_COLUMNS],
    pub col_facedown: [[u64; MAX_COL_LEN]; NUM_COLUMNS],
    pub foundation: [[u64; 14]; 4],
    pub stock_pos: [[u64; 52]; MAX_STOCK],
    pub stock_pointer: [u64; MAX_STOCK + 1],
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Zobrist {
    fn new(seed: u64) -> Self {
        let mut s = seed;
        let mut z = Zobrist {
            col_card: [[[0u64; 52]; MAX_COL_LEN]; NUM_COLUMNS],
            col_facedown: [[0u64; MAX_COL_LEN]; NUM_COLUMNS],
            foundation: [[0u64; 14]; 4],
            stock_pos: [[0u64; 52]; MAX_STOCK],
            stock_pointer: [0u64; MAX_STOCK + 1],
        };
        for c in 0..NUM_COLUMNS {
            for i in 0..MAX_COL_LEN {
                for k in 0..52 {
                    z.col_card[c][i][k] = splitmix64(&mut s);
                }
            }
        }
        for c in 0..NUM_COLUMNS {
            for i in 0..MAX_COL_LEN {
                z.col_facedown[c][i] = splitmix64(&mut s);
            }
        }
        for suit in 0..4 {
            for lvl in 0..14 {
                z.foundation[suit][lvl] = splitmix64(&mut s);
            }
        }
        for i in 0..MAX_STOCK {
            for k in 0..52 {
                z.stock_pos[i][k] = splitmix64(&mut s);
            }
        }
        for i in 0..=MAX_STOCK {
            z.stock_pointer[i] = splitmix64(&mut s);
        }
        z
    }
}

static ZOBRIST: OnceLock<Zobrist> = OnceLock::new();

/// Accès à la table globale, initialisée paresseusement une seule fois.
#[inline]
pub fn zobrist() -> &'static Zobrist {
    ZOBRIST.get_or_init(|| Zobrist::new(0xC0FF_EED1_5EA5_E511))
}
