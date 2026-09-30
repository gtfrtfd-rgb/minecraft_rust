use bevy::prelude::*;
use crate::core::state::CHUNK_SIZE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockType {
    Air,
    Grass,
    Dirt,
    Stone,
}

#[derive(Component)]
pub struct Chunk {
    pub coord: IVec2,
    pub blocks: Vec<BlockType>,
}

impl Chunk {
    pub fn new(cx: i32, cz: i32) -> Self {
        Self {
            coord: IVec2::new(cx, cz),
            blocks: vec![BlockType::Air; (CHUNK_SIZE * CHUNK_SIZE * 48) as usize],
        }
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if x >= 0 && x < CHUNK_SIZE && y >= 0 && y < 48 && z >= 0 && z < CHUNK_SIZE {
            let idx = (y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x) as usize;
            self.blocks[idx] = block;
        }
    }
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, _app: &mut App) {
        info!("ChunkPlugin loaded.");
    }
}