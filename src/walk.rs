use uuid::Uuid;

use rand::Rng;
use rand::thread_rng;


pub type WalkId = Uuid;

pub struct WalkIdGenerator {
    id: WalkId,
}

impl WalkIdGenerator {
    pub fn new() -> Self {
        WalkIdGenerator { id: Uuid::from_u128(thread_rng().gen()) }
    }

    pub fn get_id(&self) -> WalkId {
        self.id
    }
}