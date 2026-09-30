use entity::revoked_keys::{ActiveModel, Entity};
use sea_orm_migration::sea_orm::{ConnectionTrait, DbErr, EntityTrait, Set};
use std::fmt::Write;

// https://rosettacode.org/wiki/Pseudo-random_numbers/Splitmix64#Rust
// (kept verbatim; allow only silences the unused helpers)
#[allow(dead_code)]
struct Splitmix64 {
    state: u64,
    two_power_64: f64,
}

#[allow(dead_code)]
impl Splitmix64 {
    fn new() -> Self {
        Splitmix64 {
            state: 0,
            two_power_64: (2.0_f64).powi(64),
        }
    }

    fn with_seed(seed: u64) -> Self {
        Splitmix64 {
            state: seed,
            two_power_64: (2.0_f64).powi(64),
        }
    }

    fn seed(&mut self, seed: u64) {
        self.state = seed;
    }

    fn next_int(&mut self) -> u64 {
        let mut z = self.state.wrapping_add(0x9e3779b97f4a7c15);
        self.state = z;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn next_float(&mut self) -> f64 {
        self.next_int() as f64 / self.two_power_64
    }
}

const CHUNK: usize = 10_000;
const TOTAL: usize = 1_000_000;
/// 4 u64 words × 16 hex chars = 64-char key, same shape as a sha256 hex digest.
const KEY_WORDS: usize = 4;

fn key_hash(rng: &mut Splitmix64) -> String {
    let mut s = String::with_capacity(KEY_WORDS * 16);
    for _ in 0..KEY_WORDS {
        let _ = write!(s, "{:016x}", rng.next_int());
    }
    s
}

pub async fn seed<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    let mut rng = Splitmix64::with_seed(0x5EED);

    let keys: Vec<ActiveModel> = (0..TOTAL)
        .map(|_| ActiveModel {
            key_hash: Set(key_hash(&mut rng)),
            ..Default::default() // revoked_at / expires_at stay NULL
        })
        .collect();

    for chunk in keys.chunks(CHUNK) {
        Entity::insert_many(chunk.to_vec()).exec(db).await?;
    }

    Ok(())
}
