use rand::{rngs::ThreadRng, Rng};

use crate::{u32_to_color, wrap_value, ImageData};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

type Tile = Vec<u32>;

const OFFSETS: [(isize, isize); 4] = [
    (0, -1), 
    (1, 0), 
    (0, 1), 
    (-1, 0),
];
const DIRECTIONS: Range<usize> = 0..OFFSETS.len();
type Rules = [HashSet<usize>; OFFSETS.len()];

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

fn get_id(grid_ids: &[usize], x: isize, y: isize, width: usize, height: usize) -> usize {
    grid_ids[wrap_value(y, height) * width + wrap_value(x, width)]
}

#[derive(Clone)]
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

                for direction in DIRECTIONS {
                    add_rule(
                        &mut rules[id],
                        direction,
                        get_id(
                            &grid_ids,
                            x + OFFSETS[direction].0,
                            y + OFFSETS[direction].1,
                            data.width,
                            data.height,
                        ),
                    );
                }
            }
        }

        /*for (i, tile_rule) in rules.iter().enumerate() {
            eprintln!("Tile: {i}");
            for rule in tile_rule {
                for tile in rule {
                    eprint!("{tile} ");
                }
                eprintln!();
            }
            eprintln!("-------------");
        }*/

        Self {
            wfc_tiles: tiles,
            wfc_rules: rules,
            wfc_frequency: frequency,
            wfc_tile_sz: tile_sz as usize,
        }
    }

    #[allow(dead_code)]
    pub fn generate_grid(&self, w: usize, h: usize) -> ImageData {
        let mut grid = vec![0; w * h];

        let mut superpositions = {
            let id_list: Vec<usize> = (0..self.wfc_tiles.len()).collect();
            vec![id_list; w * h]
        };

        let mut rng = rand::thread_rng();

        let mut not_collapsed: Vec<usize> = (0..superpositions.len()).collect();
        let mut lowest_entropy_tiles =
            lowest_entropy(&superpositions, &not_collapsed, self.wfc_tiles.len());
        //Repeat until we have collapsed each tile into a single state
        while lowest_entropy_tiles.len() > 0 {
            //Find the tile with the lowest "entropy"
            let rand_tile_index = random_element(&lowest_entropy_tiles, &mut rng).unwrap_or(0);
            //Collapse that tile into a random state that is allowed
            superpositions[rand_tile_index] =
                vec![random_element(&superpositions[rand_tile_index], &mut rng).unwrap_or(0)];
            //Update surrounding tiles to only have valid tiles in the superposition
            let x = (rand_tile_index % w) as isize;
            let y = (rand_tile_index / w) as isize;
            //Propagate
            propagate(&mut superpositions, &self.wfc_rules, x, y, w, h);

            not_collapsed = not_collapsed
                .iter()
                .filter(|index| superpositions[**index].len() > 1)
                .map(|index| *index)
                .collect();
            lowest_entropy_tiles =
                lowest_entropy(&superpositions, &not_collapsed, self.wfc_tiles.len());
        }

        copy_superpositions_to_grid(
            &mut grid,
            &superpositions,
            &self.wfc_tiles,
            self.wfc_tile_sz,
        );

        ImageData {
            pixels: grid,
            width: w,
            height: h,
        }
    }
}

pub fn copy_superpositions_to_grid(
    grid: &mut Vec<u32>,
    superpositions: &Vec<Vec<usize>>,
    wfc_tiles: &Vec<Tile>,
    wfc_tile_sz: usize,
) {
    for i in 0..superpositions.len() {
        let center = if wfc_tile_sz % 2 == 1 {
            wfc_tile_sz / 2
        } else {
            wfc_tile_sz / 2 - 1
        };
        let tile_index = center * wfc_tile_sz + center;

        if superpositions[i].len() == 0 {
            grid[i] = 0;
            continue;
        } else if superpositions[i].len() > 1 {
            let (mut r, mut g, mut b) = (0.0f32, 0.0f32, 0.0f32);
            let mut count = 0.0f32;
            for val in &superpositions[i] {
                let col = u32_to_color(wfc_tiles[*val][tile_index]);
                r += col.r();
                g += col.g();
                b += col.b();
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

        grid[i] = wfc_tiles[superpositions[i][0]][tile_index];
    }
}

fn out_of_bounds(x: isize, y: isize, w: usize, h: usize) -> bool {
    x < 0 || y < 0 || x >= w as isize || y >= h as isize
}

pub fn update_adjacent_tiles(
    superpositions: &mut Vec<Vec<usize>>,
    x: isize,
    y: isize,
    w: usize,
    h: usize,
    rules: &Vec<Rules>,
) {
    if out_of_bounds(x, y, w, h) {
        return;
    }

    for direction in DIRECTIONS {
        let adj_x = OFFSETS[direction].0 + x;
        let adj_y = OFFSETS[direction].1 + y;

        if out_of_bounds(adj_x, adj_y, w, h) {
            continue;
        }

        let mut allowed = HashSet::<usize>::new();
        for tile in &superpositions[x as usize + y as usize * w] {
            for tile2 in &rules[*tile][direction] {
                allowed.insert(*tile2);
            }
        }

        let adj_x = adj_x as usize;
        let adj_y = adj_y as usize;
        let index = adj_x + adj_y * w;
        let mut updated = vec![];
        for tile in &superpositions[index] {
            if allowed.contains(tile) {
                updated.push(*tile);
            }
        }
        superpositions[index] = updated;
    }
}

//Returns true if no contradictions were found,
//false otherwise
pub fn propagate(
    superpositions: &mut Vec<Vec<usize>>,
    wfc_rules: &Vec<Rules>,
    x: isize,
    y: isize,
    w: usize,
    h: usize,
) -> bool {
    let mut stack = Vec::<(isize, isize)>::new();
    //Propagate the tile's properties
    stack.push((x, y));
    while !stack.is_empty() {
        let (posx, posy) = match stack.pop() {
            Some(p) => p,
            _ => return false,
        };

        let mut prev_entropy = vec![0; OFFSETS.len()];
        for direction in DIRECTIONS {
            let (adj_x, adj_y) = (posx + OFFSETS[direction].0, posy + OFFSETS[direction].1);

            if out_of_bounds(adj_x, adj_y, w, h) {
                continue;
            }

            let index = adj_x as usize + adj_y as usize * w;
            prev_entropy[direction] = superpositions[index].len();
        }

        update_adjacent_tiles(superpositions, posx, posy, w, h, wfc_rules);

        for direction in DIRECTIONS {
            let (adj_x, adj_y) = (posx + OFFSETS[direction].0, posy + OFFSETS[direction].1);

            if out_of_bounds(adj_x, adj_y, w, h) {
                continue;
            }

            let index = adj_x as usize + adj_y as usize * w;

            if superpositions[index].len() == 0 {
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

//Returns a vector of indices of elements with the lowest entropy
//This function will ignore all elements with length 1
pub fn lowest_entropy(
    superpositions: &Vec<Vec<usize>>,
    not_collapsed: &Vec<usize>,
    max_entropy: usize,
) -> Vec<usize> {
    let mut min_entropy = max_entropy;

    for i in not_collapsed {
        if superpositions[*i].len() < min_entropy {
            min_entropy = superpositions[*i].len();
        }
    }

    let mut res = vec![];
    for i in not_collapsed {
        if superpositions[*i].len() == min_entropy {
            res.push(*i);
        }
    }

    res
}

pub fn random_element<T: Copy>(vec: &Vec<T>, rng: &mut ThreadRng) -> Option<T> {
    if vec.is_empty() {
        return None;
    }
    let index = rng.gen::<usize>() % vec.len();
    Some(vec[index])
}
