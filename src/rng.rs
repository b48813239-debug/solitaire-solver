//! Petit PRNG déterministe partagé (xorshift64). Aucune prétention
//! cryptographique — sert uniquement à mélanger des tableaux de façon
//! reproductible à partir d'une graine (tests, bancs d'essai, et
//! échantillonnage de mondes en HOP).

pub struct Xorshift64(u64);

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        Xorshift64(seed ^ 0x9E3779B97F4A7C15 | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

/// Fisher-Yates in-place, déterministe à partir de `seed`.
pub fn shuffle<T>(slice: &mut [T], seed: u64) {
    let mut rng = Xorshift64::new(seed);
    for i in (1..slice.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        slice.swap(i, j);
    }
}
