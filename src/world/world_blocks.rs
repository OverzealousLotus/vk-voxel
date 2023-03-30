use std::thread;

use ahash::HashMap;
use ultraviolet::{IVec2, Vec2};

use crate::{render::renderer::Renderer, util::util::Facing};

use super::{chunk::Chunk, block_data::StaticBlockData, generation::terrain::TerrainGenerator};

pub struct WorldBlocks {
    pub loaded_chunks: HashMap<IVec2, Chunk>,
    // This should probably be a sender or something for async
    pub updated_chunks: Vec<IVec2>,
    pub terrain_generator: TerrainGenerator,
    pub player_pos: Vec2,
}

impl WorldBlocks {
    const CHUNK_UPDATES_PER_FRAME: u32 = 2;
    const RENDER_DISTANCE: u32 = 12;

    const ADJ_CHUNK_OFFSETS: [IVec2; 4] = [
        IVec2::new(1, 0),
        IVec2::new(-1, 0),
        IVec2::new(0, 1),
        IVec2::new(0, -1),
    ];

    pub fn new(block_data: &StaticBlockData) -> Self {
        Self {
            loaded_chunks: HashMap::default(),
            updated_chunks: Vec::new(),
            terrain_generator: TerrainGenerator::new_random(block_data),
            player_pos: Vec2::zero(),
        }
    }

    pub fn load_chunk(&mut self, chunk_pos: IVec2, block_data: &StaticBlockData) {
        // TODO: Load from storage
        let mut new_chunk = Chunk::generate(chunk_pos, &mut self.terrain_generator);
        new_chunk.init_mesh(block_data);

        const DIRS: [Facing; 4] = [Facing::RIGHT, Facing::LEFT, Facing::FORWARD, Facing::BACK];
        for (dir, offset) in DIRS.iter().zip(Self::ADJ_CHUNK_OFFSETS.iter()) {
            if let Some(adj_chunk) = self.loaded_chunks.get_mut(&(chunk_pos + *offset)) {
                new_chunk.cull_adjacent(*dir, adj_chunk, .., block_data);
                adj_chunk.cull_adjacent(dir.opposite(), &new_chunk, .., block_data);
                adj_chunk.rebuild_mesh(block_data);
                self.updated_chunks.push(adj_chunk.pos);
            }
        }

        new_chunk.rebuild_mesh(block_data);
        self.loaded_chunks.insert(chunk_pos, new_chunk);
        self.updated_chunks.push(chunk_pos);
    }

    pub fn frame_update(&mut self, block_data: &StaticBlockData) {
        let to_load = self.get_closest_unloaded_chunks(Self::CHUNK_UPDATES_PER_FRAME.try_into().unwrap());

        // let handle = thread::spawn(|| {

        // });

        for pos in to_load.into_iter() {
            self.load_chunk(pos, block_data);
        }

        // TODO: Unloading chunks could use a better method based on movement
        for pos in self.get_chunks_to_unload() {
            self.loaded_chunks.remove(&pos);
            self.updated_chunks.push(pos);
        }
    }

    fn get_closest_unloaded_chunks(&self, num: usize) -> Vec<IVec2> {
        let div_16 = self.player_pos / -16.0;
        let center_chunk = IVec2::new(div_16.x.floor() as i32, div_16.y.floor() as i32);
        
        let mut check = center_chunk.clone();
        let mut step = SpiralStep::Right;
        let mut steps_left = 1;
        let mut step_amount = 1;
        let mut up_step = false;

        let mut ret = Vec::new();

        while ret.len() < num {
            if let None = self.loaded_chunks.get(&check) {
                ret.push(check);
            }

            match step {
                SpiralStep::Right => check.x += 1,
                SpiralStep::Up => check.y += 1,
                SpiralStep::Left => check.x -= 1,
                SpiralStep::Down => check.y -= 1,
            }

            if (check - center_chunk).abs().component_max() as u32 > Self::RENDER_DISTANCE {
                break;
            }

            steps_left -= 1;
            if steps_left == 0 {
                if up_step { step_amount += 1; }
                up_step = !up_step;
                steps_left = step_amount;
                step.next();
            }
        }
        return ret;
    }

    fn get_chunks_to_unload(&self) -> Vec<IVec2> {
        let div_16 = self.player_pos / -16.0;
        let player_pos = IVec2::new(div_16.x.floor() as i32, div_16.y.floor() as i32);

        let mut ret = Vec::new();
        for pos in self.loaded_chunks.keys() {
            if (*pos - player_pos).abs().component_max() as u32 > Self::RENDER_DISTANCE + 1 {
                ret.push(*pos);
            }
        }
        ret
    }
}

enum SpiralStep {
    Up,
    Down,
    Left,
    Right,
}

impl SpiralStep {
    fn next(&mut self) {
        match self {
            Self::Right => *self = Self::Up,
            Self::Up => *self = Self::Left,
            Self::Left => *self = Self::Down,
            Self::Down => *self = Self::Right,
        }
    }
}