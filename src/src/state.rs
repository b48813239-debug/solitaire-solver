//! État complet d'une partie de solitaire Klondike **entièrement déterminisée**
//! (toutes les cartes, y compris celles face cachée et celles encore dans la
//! pioche, ont une identité connue). C'est la seule forme d'état que ce cœur
//! de solveur manipule : l'incertitude (cartes réellement inconnues du
//! joueur) est résolue *avant* d'entrer ici, par tirage d'un "monde" plausible
//! (couche au-dessus, pas encore écrite).
//!
//! Choix de représentation : tableaux à taille fixe, aucune allocation tas.
//! Une colonne est un tampon de cartes indexé de bas (index 0, la première
//! posée) en haut (`len-1`, la carte visible au sommet). `face_down` compte
//! combien de cartes, à partir du bas, sont encore retournées. Sortir une
//! carte du sommet ne fait que décrémenter `len` — les octets au-delà restent
//! en mémoire mais ne sont plus "vus" par le reste du code ; les remettre en
//! ré-incrémentant `len` (annulation d'un coup) est donc gratuit et correct,
//! tant qu'aucune écriture n'a eu lieu entre-temps à cette position (ce que
//! garantit la structure LIFO : on ne pousse jamais qu'à l'ancien `len`, donc
//! rien n'écrase les cartes "sous la ligne" avant que l'annulation ne les ait
//! restaurées — cf. `solver.rs`, qui n'annule jamais dans le désordre).

use crate::card::{Card, NONE};
use crate::zobrist::{zobrist, MAX_COL_LEN, MAX_STOCK, NUM_COLUMNS};

#[derive(Clone)]
pub struct Column {
    pub cards: [Card; MAX_COL_LEN],
    pub len: u8,
    pub face_down: u8,
}

impl Column {
    pub fn empty() -> Self {
        Column { cards: [NONE; MAX_COL_LEN], len: 0, face_down: 0 }
    }

    #[inline(always)]
    pub fn face_up_count(&self) -> u8 {
        self.len - self.face_down
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn top(&self) -> Option<Card> {
        if self.len > self.face_down {
            Some(self.cards[(self.len - 1) as usize])
        } else {
            None
        }
    }

    /// Carte face-up à l'index physique donné (0 = juste au-dessus du tas caché).
    #[inline(always)]
    pub fn card_at(&self, idx: u8) -> Card {
        self.cards[idx as usize]
    }
}

#[derive(Clone)]
pub struct State {
    pub columns: [Column; NUM_COLUMNS],
    /// Niveau atteint par fondation : 0 = aucune carte, 13 = complète.
    pub foundations: [u8; 4],
    pub stock: [Card; MAX_STOCK],
    pub stock_len: u8,
    /// [0, pointer) = talon (déjà tiré), [pointer, stock_len) = pioche restante.
    pub stock_pointer: u8,

    pub hash: u64,
    /// Contribution de la pioche/talon au hash total, mise en cache pour
    /// pouvoir la retirer/recalculer en O(stock_len) sans repasser sur le
    /// reste de l'état (cf. commentaire de module dans zobrist.rs).
    stock_hash: u64,
}

impl State {
    /// Construit un état à partir d'un monde entièrement déterminisé.
    /// `column_cards[i]` va du bas vers le haut ; `column_facedown[i]` cartes,
    /// à partir du bas, démarrent cachées. `stock_cards` est l'ordre de pioche
    /// (index 0 = première carte tirée).
    pub fn deal(
        column_cards: &[Vec<Card>; NUM_COLUMNS],
        column_facedown: &[u8; NUM_COLUMNS],
        stock_cards: &[Card],
    ) -> State {
        let mut columns: [Column; NUM_COLUMNS] = std::array::from_fn(|_| Column::empty());
        for i in 0..NUM_COLUMNS {
            let src = &column_cards[i];
            assert!(src.len() <= MAX_COL_LEN, "colonne trop longue");
            assert!((column_facedown[i] as usize) <= src.len());
            for (idx, &c) in src.iter().enumerate() {
                columns[i].cards[idx] = c;
            }
            columns[i].len = src.len() as u8;
            columns[i].face_down = column_facedown[i];
        }

        let mut stock = [NONE; MAX_STOCK];
        assert!(stock_cards.len() <= MAX_STOCK, "pioche trop longue");
        for (i, &c) in stock_cards.iter().enumerate() {
            stock[i] = c;
        }

        let mut state = State {
            columns,
            foundations: [0; 4],
            stock,
            stock_len: stock_cards.len() as u8,
            stock_pointer: 0,
            hash: 0,
            stock_hash: 0,
        };
        state.recompute_hash_from_scratch();
        state
    }

    pub fn is_won(&self) -> bool {
        self.foundations.iter().all(|&lvl| lvl == 13)
    }

    /// Nombre total de cartes déjà en fondation (mesure de progression, 0..52).
    pub fn foundation_count(&self) -> u32 {
        self.foundations.iter().map(|&l| l as u32).sum()
    }

    pub fn stock_top(&self) -> Option<Card> {
        if self.stock_pointer > 0 {
            Some(self.stock[(self.stock_pointer - 1) as usize])
        } else {
            None
        }
    }

    // ---- Primitives de mutation, chacune maintenant `hash` de façon incrémentale ----

    #[inline]
    pub fn col_push(&mut self, col: usize, card: Card) {
        let z = zobrist();
        let c = &mut self.columns[col];
        let idx = c.len as usize;
        c.cards[idx] = card;
        c.len += 1;
        self.hash ^= z.col_card[col][idx][card as usize];
    }

    /// Retire la carte au sommet (ne vérifie PAS le retournement ; voir `maybe_reveal`).
    #[inline]
    pub fn col_pop(&mut self, col: usize) -> Card {
        let z = zobrist();
        let c = &mut self.columns[col];
        c.len -= 1;
        let idx = c.len as usize;
        let card = c.cards[idx];
        self.hash ^= z.col_card[col][idx][card as usize];
        card
    }

    /// À appeler après un ou plusieurs `col_pop` : si la colonne n'a plus de
    /// carte face visible et qu'il reste des cartes cachées, retourne la
    /// nouvelle carte du dessus. Retourne `true` si un retournement a eu lieu.
    #[inline]
    pub fn maybe_reveal(&mut self, col: usize) -> bool {
        let z = zobrist();
        let c = &mut self.columns[col];
        if c.len == c.face_down && c.face_down > 0 {
            let old_fd = c.face_down as usize;
            self.hash ^= z.col_facedown[col][old_fd];
            c.face_down -= 1;
            let new_fd = c.face_down as usize;
            self.hash ^= z.col_facedown[col][new_fd];
            let revealed_card = c.cards[new_fd];
            self.hash ^= z.col_card[col][new_fd][revealed_card as usize];
            true
        } else {
            false
        }
    }

    /// Inverse exact de `maybe_reveal` (utilisé lors de l'annulation d'un coup).
    #[inline]
    pub fn unreveal(&mut self, col: usize) {
        let z = zobrist();
        let c = &mut self.columns[col];
        let fd = c.face_down as usize;
        let revealed_card = c.cards[fd];
        self.hash ^= z.col_card[col][fd][revealed_card as usize];
        self.hash ^= z.col_facedown[col][fd];
        c.face_down += 1;
        self.hash ^= z.col_facedown[col][c.face_down as usize];
    }

    #[inline]
    pub fn set_foundation(&mut self, suit: usize, new_level: u8) {
        let z = zobrist();
        let old = self.foundations[suit] as usize;
        self.hash ^= z.foundation[suit][old];
        self.foundations[suit] = new_level;
        self.hash ^= z.foundation[suit][new_level as usize];
    }

    #[inline]
    pub fn draw(&mut self) {
        let z = zobrist();
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
        self.stock_pointer += 1;
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
    }

    #[inline]
    pub fn undraw(&mut self) {
        let z = zobrist();
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
        self.stock_pointer -= 1;
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
    }

    #[inline]
    pub fn recycle(&mut self) {
        let z = zobrist();
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
        self.stock_pointer = 0;
        self.hash ^= z.stock_pointer[0];
    }

    #[inline]
    pub fn unrecycle(&mut self, old_pointer: u8) {
        let z = zobrist();
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
        self.stock_pointer = old_pointer;
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
    }

    /// Retire définitivement la carte au sommet du talon (jouée vers une
    /// colonne ou une fondation) et referme le trou : les cartes non encore
    /// tirées glissent d'un cran. Coût O(stock_len), borné à 24 — négligeable
    /// et bien plus simple qu'un hash incrémental "au milieu d'un tableau".
    #[inline]
    pub fn stock_remove_top(&mut self) -> Card {
        let z = zobrist();
        self.retract_stock_hash();
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
        let p = (self.stock_pointer - 1) as usize;
        let card = self.stock[p];
        for i in p..(self.stock_len as usize - 1) {
            self.stock[i] = self.stock[i + 1];
        }
        self.stock[self.stock_len as usize - 1] = NONE;
        self.stock_len -= 1;
        self.stock_pointer -= 1;
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
        self.apply_stock_hash();
        card
    }

    /// Inverse de `stock_remove_top` : réinsère `card` à la position qu'elle
    /// occupait (juste avant l'ancien pointeur).
    #[inline]
    pub fn stock_reinsert(&mut self, card: Card, old_pointer: u8, old_len: u8) {
        let z = zobrist();
        self.retract_stock_hash();
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
        let insert_at = (old_pointer - 1) as usize;
        let mut i = old_len as usize - 1;
        while i > insert_at {
            self.stock[i] = self.stock[i - 1];
            i -= 1;
        }
        self.stock[insert_at] = card;
        self.stock_len = old_len;
        self.stock_pointer = old_pointer;
        self.hash ^= z.stock_pointer[self.stock_pointer as usize];
        self.apply_stock_hash();
    }

    fn retract_stock_hash(&mut self) {
        self.hash ^= self.stock_hash;
    }

    fn apply_stock_hash(&mut self) {
        self.stock_hash = Self::compute_stock_hash(&self.stock, self.stock_len);
        self.hash ^= self.stock_hash;
    }

    fn compute_stock_hash(stock: &[Card; MAX_STOCK], stock_len: u8) -> u64 {
        let z = zobrist();
        let mut h = 0u64;
        for i in 0..stock_len as usize {
            h ^= z.stock_pos[i][stock[i] as usize];
        }
        h
    }

    /// Recalcule tout le hash depuis zéro (utilisé une seule fois, à la
    /// construction d'un monde ; jamais dans la boucle chaude du solveur).
    pub fn recompute_hash_from_scratch(&mut self) {
        let z = zobrist();
        let mut h = 0u64;
        for col in 0..NUM_COLUMNS {
            let c = &self.columns[col];
            h ^= z.col_facedown[col][c.face_down as usize];
            for idx in (c.face_down as usize)..(c.len as usize) {
                h ^= z.col_card[col][idx][c.cards[idx] as usize];
            }
        }
        for suit in 0..4 {
            h ^= z.foundation[suit][self.foundations[suit] as usize];
        }
        self.stock_hash = Self::compute_stock_hash(&self.stock, self.stock_len);
        h ^= self.stock_hash;
        h ^= z.stock_pointer[self.stock_pointer as usize];
        self.hash = h;
    }
}
