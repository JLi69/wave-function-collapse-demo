use rand::{rngs::ThreadRng, Rng};

use crate::{wrap_value, ImageData};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::Range,
};

type Tile = Vec<u32>;
type Rules = [HashSet<usize>; 4];

const UP: usize = 0;
const RIGHT: usize = 1;
const DOWN: usize = 2;
const LEFT: usize = 3;
const DIRECTIONS: Range<usize> = UP..(LEFT + 1);
const OFFSETS: [(isize, isize); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

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

                add_rule(
                    &mut rules[id],
                    UP,
                    get_id(&grid_ids, x, y - 1, data.width, data.height),
                );
                add_rule(
                    &mut rules[id],
                    RIGHT,
                    get_id(&grid_ids, x + 1, y, data.width, data.height),
                );
                add_rule(
                    &mut rules[id],
                    DOWN,
                    get_id(&grid_ids, x, y + 1, data.width, data.height),
                );
                add_rule(
                    &mut rules[id],
                    LEFT,
                    get_id(&grid_ids, x - 1, y, data.width, data.height),
                );
            }
        }

        Self {
            wfc_tiles: tiles,
            wfc_rules: rules,
            wfc_frequency: frequency,
            wfc_tile_sz: tile_sz as usize,
        }
    }

    pub fn generate_grid(&self, w: usize, h: usize) -> ImageData {
        let mut grid = vec![0; w * h];

        let mut superpositions = {
            let id_list: Vec<usize> = (0..self.wfc_tiles.len()).collect();
            vec![id_list; w * h]
        };

        let mut rng = rand::thread_rng();

        let mut not_collapsed: Vec<usize> = (0..superpositions.len()).collect();
        let mut collapsed_set = HashSet::<usize>::new();
        let mut lowest_entropy_tiles =
            lowest_entropy(&superpositions, &not_collapsed, self.wfc_tiles.len());
        let mut queue = VecDeque::<(isize, isize)>::new();
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
            update_adjacent_tiles(&mut superpositions, x, y, w, h, &self.wfc_rules);
            collapsed_set.insert(rand_tile_index);

            //Propagate the tile's properties
            let mut visited = vec![false; w * h];
            queue.push_back((x, y));
            while !queue.is_empty() {
                let (posx, posy) = queue[0];
                queue.pop_front();

                if superpositions[posx as usize + posy as usize * w].len() <= 1 {
                    collapsed_set.insert(posx as usize + posy as usize * w);
                }

                if visited[posx as usize + posy as usize * w] {
                    continue;
                }

                visited[posx as usize + posy as usize * w] = true;
                update_adjacent_tiles(&mut superpositions, posx, posy, w, h, &self.wfc_rules);
                for direction in DIRECTIONS {
                    let (adj_x, adj_y) = (posx + OFFSETS[direction].0, posy + OFFSETS[direction].1);

                    if out_of_bounds(adj_x, adj_y, w, h) {
                        continue;
                    }

                    let index = adj_x as usize + adj_y as usize * w;

                    if superpositions[index].len() == self.wfc_tiles.len()
                        || superpositions[index].len() == 0
                    {
                        continue;
                    }

                    if visited[index] {
                        continue;
                    }

                    if collapsed_set.contains(&(adj_x as usize + adj_y as usize * h)) {
                        continue;
                    }

                    queue.push_back((adj_x, adj_y));
                }
            }

            not_collapsed = not_collapsed
                .iter()
                .filter(|index| superpositions[**index].len() > 1)
                .map(|index| *index)
                .collect();
            lowest_entropy_tiles =
                lowest_entropy(&superpositions, &not_collapsed, self.wfc_tiles.len());
        }

        for y in 0..h {
            for x in 0..w {
                let index = y * w + x;
                let center = if self.wfc_tile_sz % 2 == 1 {
                    self.wfc_tile_sz / 2
                } else {
                    self.wfc_tile_sz / 2 - 1
                };
                let tile_index = center * self.wfc_tile_sz + center;

                if superpositions[index].is_empty() {
                    continue;
                }

                grid[index] = self.wfc_tiles[superpositions[index][0]][tile_index];
            }
        }

        ImageData {
            pixels: grid,
            width: w,
            height: h,
        }
    }
}

fn out_of_bounds(x: isize, y: isize, w: usize, h: usize) -> bool {
    x < 0 || y < 0 || x >= w as isize || y >= h as isize
}

fn update_adjacent_tiles(
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

        let adj_x = adj_x as usize;
        let adj_y = adj_y as usize;
        let index = adj_x + adj_y * w;

        let mut allowed = HashSet::<usize>::new();
        for tile in &superpositions[x as usize + y as usize * w] {
            for tile2 in &rules[*tile][direction] {
                allowed.insert(*tile2);
            }
        }

        let mut updated = vec![];
        for tile in &superpositions[index] {
            if allowed.contains(tile) {
                updated.push(*tile);
            } 
        }

        superpositions[index] = updated;
    }
}

//Returns a vector of indices of elements with the lowest entropy
//This function will ignore all elements with length 1
fn lowest_entropy(
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

fn random_element<T: Copy>(vec: &Vec<T>, rng: &mut ThreadRng) -> Option<T> {
    if vec.is_empty() {
        return None;
    }
    let index = rng.gen::<usize>() % vec.len();
    Some(vec[index])
}
