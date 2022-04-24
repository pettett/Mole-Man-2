use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::BufReader,
    ops::{Index, IndexMut},
};

use bevy_ecs::prelude as ecs;
use rand::{prelude::IteratorRandom, Rng};

use super::{Orientation, TileRequirements};
use serde::{
    de::{self, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::str::FromStr;
///Store config for a tilemap sprite
#[derive(ecs::Component, Serialize, Deserialize)]
pub struct TilemapSpriteConfig {
    ///Valid placements for tile (usize,usize)
    pub orientations: HashMap<GridCoordinate, TileRequirements>,

    #[serde(skip)]
    coordinates: CoordinateSet,

    /// Amount of tiles horizontally
    pub grid_width: usize,
    pub grid_height: usize,

    ///Width of a tile (sub-sprite) inside the grid
    pub tile_width: usize,
    pub tile_height: usize,
}

struct CoordinateSet {
    // store coordinate for all possible orientations
    coordinates: HashMap<u8, HashSet<GridCoordinate>>,
}
impl Default for CoordinateSet {
    fn default() -> Self {
        Self {
            coordinates: Default::default(),
        }
    }
}

impl CoordinateSet {
    pub fn insert(&mut self, k: Orientation, v: GridCoordinate) {
        if let Some(s) = self.coordinates.get_mut(&k.bits) {
            s.insert(v);
        } else {
            self.coordinates.insert(k.bits, [v].into());
        }
    }

    fn get(&self, index: Orientation) -> Option<GridCoordinate> {
        let mut rng = rand::thread_rng();

        if let Some(s) = self.coordinates.get(&(index.bits)) {
            if let Some(c) = s.iter().choose(&mut rng) {
                Some(*c)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct GridCoordinate {
    pub x: usize,
    pub y: usize,
}

impl From<(usize, usize)> for GridCoordinate {
    fn from(c: (usize, usize)) -> Self {
        GridCoordinate::new(c.0, c.1)
    }
}

impl From<GridCoordinate> for (usize, usize) {
    fn from(c: GridCoordinate) -> Self {
        (c.x, c.y)
    }
}

impl GridCoordinate {
    pub fn new(x: usize, y: usize) -> GridCoordinate {
        GridCoordinate { x, y }
    }
}

impl Serialize for GridCoordinate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}:{}", self.x, self.y))
    }
}

struct GridCoordinateVisitor;

impl<'de> Visitor<'de> for GridCoordinateVisitor {
    type Value = GridCoordinate;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a colon-separated pair of integers between 0 and usize::MAX")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let mut nums = s.split(":");

        if let Ok(x) = usize::from_str(&nums.next().unwrap()) {
            // nums[0] is the whole match, so we must skip that
            if let Ok(y) = usize::from_str(&nums.next().unwrap()) {
                Ok(GridCoordinate::new(x, y))
            } else {
                Err(de::Error::invalid_value(Unexpected::Str(s), &self))
            }
        } else {
            Err(de::Error::invalid_value(Unexpected::Str(s), &self))
        }
    }
}

impl<'de> Deserialize<'de> for GridCoordinate {
    fn deserialize<D>(deserializer: D) -> Result<GridCoordinate, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(GridCoordinateVisitor)
    }
}

impl TilemapSpriteConfig {
    pub fn new_or_load(asset: &'static str, grid_width: usize, grid_height: usize) -> Self {
        let mut config = match std::fs::File::open(asset) {
            Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
            Err(_) => TilemapSpriteConfig::new(grid_width, grid_height),
        };

        config.sync_coordinates();

        config
    }

    pub fn save(&self, asset: &'static str) {
        let json = serde_json::to_string(self).unwrap();
        std::fs::write(asset, json).unwrap();
    }

    pub fn new(grid_width: usize, grid_height: usize) -> Self {
        Self {
            tile_width: 8,
            tile_height: 8,
            grid_width,
            grid_height,
            orientations: Default::default(),
            coordinates: Default::default(),
        }
    }

    pub fn tile_size_uv(&self) -> [f32; 2] {
        [1.0 / self.grid_width as f32, 1.0 / self.grid_height as f32]
    }

    pub fn grid_width(&self) -> u32 {
        self.grid_width as u32
    }

    pub fn position_uv(&self, x: usize, y: usize) -> ([f32; 2], [f32; 2]) {
        let [tile_width, tile_height] = self.tile_size_uv();

        (
            [tile_width * x as f32, tile_height * y as f32],
            [tile_width * (x + 1) as f32, tile_height * (y + 1) as f32],
        )
    }

    /// Search the spritemap for this orientation of tile,
    /// starting at the most specific and getting progressively more vague
    pub fn find_tile_index(&self, o: Orientation) -> Option<GridCoordinate> {
        //Search the sprite data for this orientation,
        self.coordinates.get(o)
    }
    /// Sync the coordinates array to the orientations hashmap
    pub fn sync_coordinates(&mut self) {
        for (k, v) in &mut self.orientations {
            let mut coords = Vec::new();
            //Push the first coordinate
            coords.push(*v);

            //for each cardinal direction
            for i in 0..8 {
                //For any bits that are null, double the size of the vector
                if v.dirs[i].is_none() {
                    let mut n_coords = Vec::with_capacity(coords.len() * 2);

                    for c in coords {
                        let mut dirs = c.dirs;

                        dirs[i] = Some(true);
                        n_coords.push(TileRequirements { dirs });

                        dirs[i] = Some(false);
                        n_coords.push(TileRequirements { dirs });
                    }

                    coords = n_coords;
                }
            }
            for c in coords {
                // Add to the set of coords that are valid in this orientation
                self.coordinates.insert(c.into(), *k);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialisation() {
        let key = GridCoordinate::new(4, 3);
        let value = "a steep hill";

        let mut map: HashMap<GridCoordinate, &str> = HashMap::new();
        map.insert(key, value);
        let serialised = serde_json::to_string(&map).unwrap();

        assert_eq!(serialised, r#"{"4:3":"a steep hill"}"#);
    }

    #[test]
    fn deserialisation() {
        let json = r#"{"4:3":"a steep hill"}"#;

        let deserialised: HashMap<GridCoordinate, &str> = serde_json::from_str(&json).unwrap();

        let key = GridCoordinate::new(4, 3);
        let value = "a steep hill";

        let mut map: HashMap<GridCoordinate, &str> = HashMap::new();
        map.insert(key, value);

        assert_eq!(deserialised, map);
    }

    #[test]
    fn sync_coordinates() {
        let mut c = TilemapSpriteConfig::new(4, 4);

        c.orientations.insert(
            (2, 2).into(),
            TileRequirements {
                dirs: [
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                ],
            },
        );

        c.orientations.insert(
            (2, 1).into(),
            TileRequirements {
                dirs: [
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                ],
            },
        );
        // Should insert into NONE and S
        c.orientations.insert(
            (1, 1).into(),
            TileRequirements {
                dirs: [
                    Some(false),
                    None,
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                ],
            },
        );

        c.sync_coordinates();

        assert!(c.find_tile_index(Orientation::all()).is_some());

        assert!(c.find_tile_index(Orientation::NONE).is_some());
        assert!(c.find_tile_index(Orientation::S).is_some());

        // Nothing has set this
        assert!(c.find_tile_index(Orientation::N).is_none());
    }
}
