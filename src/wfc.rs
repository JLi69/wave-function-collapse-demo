use crate::ImageData;
use std::collections::{HashMap, HashSet};

type Tile = Vec<u32>;
type Rules = [HashSet<usize>; 4];

const UP: usize = 0;
const RIGHT: usize = 1;
const DOWN: usize = 2;
const LEFT: usize = 3;

fn empty_rule() -> Rules {
    [
        HashSet::new(),
        HashSet::new(),
        HashSet::new(),
        HashSet::new(),
    ]
}

fn sample_square(data: &ImageData, tile_sz: isize, tile_x: isize, tile_y: isize) -> Tile {
    let mut tile = vec![0; (tile_sz * tile_sz) as usize];

    if tile_sz % 2 == 1 {
        for y in (tile_y - tile_sz / 2)..(tile_y + tile_sz / 2 + 1) {
            for x in (tile_x - tile_sz / 2)..(tile_x + tile_sz / 2 + 1) {
                let ind = ((x - (tile_x - tile_sz / 2)) + (y - (tile_y - tile_sz / 2)) * tile_sz)
                    as usize;
                tile[ind] = data.get_pixel_wrap(x, y);
            }
        }
    } else if tile_sz % 2 == 0 {
        for y in (tile_y - tile_sz / 2)..(tile_y + tile_sz / 2) {
            for x in (tile_x - tile_sz / 2)..(tile_x + tile_sz / 2) {
                let ind = ((x - (tile_x - tile_sz / 2)) + (y - (tile_y - tile_sz / 2)) * tile_sz)
                    as usize;
                tile[ind] = data.get_pixel_wrap(x, y);
            }
        }
    }

    tile
}

fn add_rule(rules: &mut Rules, direction: usize, id: usize) {
    rules[direction].insert(id);
}

fn get_id(grid_ids: &[usize], x: isize, y: isize, width: isize, height: isize) -> Option<usize> {
    if x < 0 || y < 0 || x >= width || y >= height {
        return None;
    }

    Some(grid_ids[(y * width + x) as usize])
}

pub struct WFCParameters {
    pub wfc_tiles: Vec<Tile>,
    pub wfc_rules: Vec<Rules>,
    pub wfc_frequency: Vec<u32>,
    pub wfc_tile_sz: usize,
}

impl WFCParameters {
    //Sample all possible tile_sz x tile_sz square regions of the image
    //and count their frequency and what they are adjacent to,
    //also assign a usize id to each one
    pub fn from_image_data(data: &ImageData, tile_sz: isize) -> Self {
        let mut id: usize = 0;
        let mut tile_ids = HashMap::<Tile, usize>::new();
        let mut tiles = Vec::<Tile>::new();
        let mut rules = Vec::<Rules>::new();
        let mut frequency = Vec::<u32>::new();
        let mut grid_ids = vec![0usize; data.width * data.height];
        for y in 0..data.height {
            for x in 0..data.width {
                let tile = sample_square(data, tile_sz, x as isize, y as isize);
    
                match tile_ids.get(&tile) {
                    Some(i) => {
                        grid_ids[y * data.width + x] = *i;
                        frequency[*i] += 1;
                    }
                    None => {
                        tile_ids.insert(tile.clone(), id);
                        grid_ids[y * data.width + x] = id;
                        id += 1;
                        tiles.push(tile.clone());
                        rules.push(empty_rule());
                        frequency.push(1);
                    }
                }
            }
        }
    
        for y in 0..(data.height as isize) {
            for x in 0..(data.width as isize) {
                let id = grid_ids[y as usize * data.width + x as usize];
    
                if let Some(adj) = get_id(
                    &grid_ids,
                    x,
                    y - 1,
                    data.width as isize,
                    data.height as isize,
                ) {
                    add_rule(&mut rules[id], UP, adj);
                }
                if let Some(adj) = get_id(
                    &grid_ids,
                    x + 1,
                    y,
                    data.width as isize,
                    data.height as isize,
                ) {
                    add_rule(&mut rules[id], RIGHT, adj);
                }
                if let Some(adj) = get_id(
                    &grid_ids,
                    x,
                    y + 1,
                    data.width as isize,
                    data.height as isize,
                ) {
                    add_rule(&mut rules[id], DOWN, adj);
                }
                if let Some(adj) = get_id(
                    &grid_ids,
                    x - 1,
                    y,
                    data.width as isize,
                    data.height as isize,
                ) {
                    add_rule(&mut rules[id], LEFT, adj);
                }
            }
        }
        
        Self { 
            wfc_tiles: tiles, 
            wfc_rules: rules, 
            wfc_frequency: frequency, 
            wfc_tile_sz: tile_sz as usize
        }
    }
}
