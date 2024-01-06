use crate::{image_data::u32_to_color, image_data::ImageData, image_data::wrap_value};
use rand::{rngs::ThreadRng, Rng};
use std::collections::HashMap;

type Tile = Vec<u32>;
const OFFSETS: [(isize, isize); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

fn sample_square(data: &ImageData, tile_sz: isize, tile_x: isize, tile_y: isize) -> Tile {
    let mut tile = vec![0; (tile_sz * tile_sz) as usize];

    for y in tile_y..(tile_y + tile_sz) {
        for x in tile_x..(tile_x + tile_sz) {
            let ind = ((x - tile_x) + (y - tile_y) * tile_sz) as usize;
            tile[ind] = data.get_pixel_wrap(x, y);
        }
    }

    tile
}

#[derive(Clone)]
pub struct RuleTable {
    rules: Vec<bool>,
    tile_count: usize,
}

impl RuleTable {
    fn new(count: usize) -> Self {
        Self {
            rules: vec![false; count * count * OFFSETS.len()],
            tile_count: count,
        }
    }

    fn add_rule(&mut self, direction: usize, id1: usize, id2: usize) {
        self.rules[id1 * self.tile_count * OFFSETS.len() + direction * self.tile_count + id2] =
            true;
    }

    fn okay(&self, direction: usize, id1: usize, id2: usize) -> bool {
        self.rules[id1 * self.tile_count * OFFSETS.len() + direction * self.tile_count + id2]
    }
}

fn tiles_match(
    tile1: &Tile,
    tile2: &Tile,
    offset_x: isize,
    offset_y: isize,
    tile_sz: isize,
) -> bool {
    for y in 0..tile_sz {
        for x in 0..tile_sz {
            let offset_x = x - offset_x;
            let offset_y = y - offset_y;

            if offset_x < 0 || offset_y < 0 || offset_x >= tile_sz || offset_y >= tile_sz {
                continue;
            }

            let index = (y * tile_sz + x) as usize;
            let offset_index = (offset_y * tile_sz + offset_x) as usize;

            if tile1[index] != tile2[offset_index] {
                return false;
            }
        }
    }

    true
}

#[derive(Clone)]
pub struct WFCParameters {
    pub wfc_tiles: Vec<u32>,
    pub wfc_rules: RuleTable,
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
        let mut frequency = Vec::<u32>::new();
        for y in 0..data.height() {
            for x in 0..data.width() {
                let tile = sample_square(data, tile_sz, x as isize, y as isize);

                match tile_ids.get(&tile) {
                    Some(i) => {
                        frequency[*i] += 1;
                    }
                    None => {
                        tile_ids.insert(tile.clone(), id);
                        tiles.push(tile.clone());
                        frequency.push(1);
                        id += 1;
                    }
                }
            }
        }

        let mut rules = RuleTable::new(tiles.len());

        for (id1, tile1) in tiles.iter().enumerate() {
            for (id2, tile2) in tiles.iter().enumerate() {
                for (direction, offset) in OFFSETS.iter().enumerate() {
                    if tiles_match(tile1, tile2, offset.0, offset.1, tile_sz) {
                        rules.add_rule(direction, id1, id2);
                    }
                }
            }
        }

        Self {
            wfc_tiles: tiles.iter().map(|tile| tile[0]).collect(),
            wfc_rules: rules,
            wfc_frequency: frequency,
            wfc_tile_sz: tile_sz as usize,
        }
    }

    #[allow(dead_code)]
    pub fn generate_grid(&self, w: usize, h: usize) -> Result<ImageData, String> {
        let mut grid = vec![0; w * h];

        let mut superpositions = {
            let id_list: Vec<usize> = (0..self.wfc_tiles.len()).collect();
            vec![id_list; w * h]
        };

        let mut rng = rand::thread_rng();

        let mut not_collapsed: Vec<usize> = (0..superpositions.len()).collect();
        let mut lowest_entropy_tiles =
            lowest_entropy(&superpositions, &not_collapsed, &self.wfc_frequency);
        //Repeat until we have collapsed each tile into a single state
        while !lowest_entropy_tiles.is_empty() {
            //Find the tile with the lowest "entropy"
            let rand_tile_index = random_element(&lowest_entropy_tiles, &mut rng, None).unwrap_or(0);

            let weights: Vec<u32> = superpositions[rand_tile_index].iter()
                .map(|tile| self.wfc_frequency[*tile])
                .collect();

            //Collapse that tile into a random state that is allowed
            superpositions[rand_tile_index] =
                vec![random_element(&superpositions[rand_tile_index], &mut rng, Some(&weights)).unwrap_or(0)];
            //Update surrounding tiles to only have valid tiles in the superposition
            let x = (rand_tile_index % w) as isize;
            let y = (rand_tile_index / w) as isize;
            //Propagate
            let failed = propagate(&mut superpositions, &self.wfc_rules, x, y, w, h);
            if failed {
                return Err("WFC Failed".to_string());
            }

            not_collapsed.retain(|index| superpositions[*index].len() > 1);
            lowest_entropy_tiles =
                lowest_entropy(&superpositions, &not_collapsed, &self.wfc_frequency);
        }

        copy_superpositions_to_grid(&mut grid, &superpositions, &self.wfc_tiles);

        Ok(ImageData::from_pixels(&grid, w, h))
    }
}

pub fn copy_superpositions_to_grid(
    grid: &mut [u32],
    superpositions: &[Vec<usize>],
    wfc_tiles: &[u32],
) {
    for i in 0..superpositions.len() {
        if superpositions[i].is_empty() {
            grid[i] = 0;
            continue;
        } else if superpositions[i].len() > 1 {
            let (mut r, mut g, mut b) = (0.0f32, 0.0f32, 0.0f32);
            let mut count = 0.0f32;
            for val in &superpositions[i] {
                let col = u32_to_color(wfc_tiles[*val]);
                r += col.0;
                g += col.1;
                b += col.2;
                count += 1.0;
            }
            let (avg_r, avg_g, avg_b) = (r / count, g / count, b / count);
            let (avg_r, avg_g, avg_b) = (
                (avg_r * 255.0) as u32,
                (avg_g * 255.0) as u32,
                (avg_b * 255.0) as u32,
            );
            grid[i] = avg_b << 16 | avg_g << 8 | avg_r | 0xff << 24;
            continue;
        }

        grid[i] = wfc_tiles[superpositions[i][0]];
    }
}

pub fn update_adjacent_tiles(
    superpositions: &mut [Vec<usize>],
    x: isize,
    y: isize,
    w: usize,
    h: usize,
    rules: &RuleTable,
) {
    for (direction, offset) in OFFSETS.iter().enumerate() {
        let adj_x = wrap_value(offset.0 + x, w) as isize;
        let adj_y = wrap_value(offset.1 + y, h) as isize; 

        let mut allowed = vec![false; rules.tile_count];
        for tile in &superpositions[x as usize + y as usize * w] {
            for (tile2, rule) in allowed.iter_mut().enumerate().take(rules.tile_count) {
                *rule = *rule || rules.okay(direction, *tile, tile2)
            }
        }

        let adj_x = adj_x as usize;
        let adj_y = adj_y as usize;
        let index = adj_x + adj_y * w;
        let mut updated = vec![];
        for tile in &superpositions[index] {
            if allowed[*tile] {
                updated.push(*tile);
            }
        }
        superpositions[index] = updated;
    }
}

//Returns true if no contradictions were found,
//false otherwise
pub fn propagate(
    superpositions: &mut [Vec<usize>],
    wfc_rules: &RuleTable,
    x: isize,
    y: isize,
    w: usize,
    h: usize,
) -> bool {
    let mut stack = Vec::<(isize, isize)>::new();
    let mut prev_entropy = vec![0; OFFSETS.len()];
    //Propagate the tile's properties
    stack.push((x, y));
    while !stack.is_empty() {
        let (posx, posy) = match stack.pop() {
            Some(p) => p,
            _ => return false,
        };

        for direction in 0..OFFSETS.len() {
            let (adj_x, adj_y) = (
                wrap_value(posx + OFFSETS[direction].0, w), 
                wrap_value(posy + OFFSETS[direction].1, h)
            );

            let index = adj_x + adj_y * w;
            prev_entropy[direction] = superpositions[index].len();
        }

        update_adjacent_tiles(superpositions, posx, posy, w, h, wfc_rules);

        for direction in 0..OFFSETS.len() {
            let (adj_x, adj_y) = (
                wrap_value(posx + OFFSETS[direction].0, w) as isize,
                wrap_value(posy + OFFSETS[direction].1, h) as isize
            );

            let index = adj_x as usize + adj_y as usize * w;

            if superpositions[index].is_empty() {
                return true;
            }

            if superpositions[index].len() == prev_entropy[direction] {
                continue;
            }

            stack.push((adj_x, adj_y));
        }
    }

    false
}

fn entropy(superposition: &[usize], frequencies: &[u32]) -> f32 {
    let mut total = 0;
    for value in superposition {
        total += frequencies[*value];
    }

    let mut res = 0.0;
    for value in superposition {
        let prob = frequencies[*value] as f32 / total as f32;
        res += prob * -prob.log2();
    }
    res
}

//Returns a vector of indices of elements with the lowest entropy
//This function will ignore all elements with length 1
pub fn lowest_entropy(
    superpositions: &[Vec<usize>],
    not_collapsed: &[usize],
    frequencies: &[u32],
) -> Vec<usize> {
    if not_collapsed.is_empty() {
        return vec![];
    }

    let mut min_entropy = entropy(&superpositions[not_collapsed[0]], frequencies);

    let mut cached_entropy = vec![0.0f32; not_collapsed.len()];
    for (i, tile) in not_collapsed.iter().enumerate() {
        let tile_entropy = entropy(&superpositions[*tile], frequencies);
        if tile_entropy < min_entropy {
            min_entropy = tile_entropy;
        }
        cached_entropy[i] = tile_entropy;
    }

    let mut res = vec![];
    for (i, tile) in not_collapsed.iter().enumerate() {
        if cached_entropy[i] == min_entropy {
            res.push(*tile);
        }
    }

    res
}

fn generate_weighted(rng: &mut ThreadRng, weights: &[u32]) -> usize {
    if weights.is_empty() {
        return 0;
    }

    let mut total = 0;
    for v in weights {
        total += v;
    }
    let rand_value = rng.gen::<u32>() % total;
    
    let mut current_total = 0;
    for (i, v) in weights.iter().enumerate() { 
        current_total += v;
        if rand_value < current_total {
            return i;
        }
    }

    weights.len() - 1
}

pub fn random_element<T: Copy>(vec: &[T], rng: &mut ThreadRng, weights: Option<&[u32]>) -> Option<T> {
    if vec.is_empty() {
        return None;
    }

    match weights {
        Some(weight_list) => {
            Some(vec[generate_weighted(rng, weight_list)])
        }
        _ => {
            let index = rng.gen::<usize>() % vec.len();
            Some(vec[index])
        }
    }
}
