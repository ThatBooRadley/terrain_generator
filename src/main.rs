use std::io::Write;

const SIZE: (usize, usize) = (64, 64);
const WATERLEVEL: i32 = (HEIGHTMAX.0 - HEIGHTMAX.1) / 8;
const HEIGHTMAX: (i32, i32) = (128, -128);
const EFFICIENCYSCALE: usize = HeightMap::next_prime(if SIZE.0 > SIZE.1 {SIZE.0} else {SIZE.1});
fn main() {
    print!("> ");
    let _ = std::io::stdout().flush();
    let mut seed = String::new();
    let _ = std::io::stdin().read_line(&mut seed);

    let height_map = HeightMap::new(seed.to_string());
    height_map.display();
    println!("seed: {}", seed.to_string());
}

struct HeightMap{
    size: (usize, usize),
    max: i32,
    min: i32,
    water_level: i32,
    seed: Option<usize>,

    map: Vec<Vec<i32>>,
}

impl Default for HeightMap {
    fn default() -> Self {
        HeightMap {
            size: SIZE,
            max: HEIGHTMAX.0,
            min: HEIGHTMAX.1,
            water_level: WATERLEVEL,
            seed: None,

            map: empty_matrix(SIZE.0, SIZE.1),
        }
    }
}

impl HeightMap {
    fn new(seed: String) -> Self {
        let mut height_map = HeightMap::default();

        let mut total = 0;
        seed.as_bytes().iter().enumerate().for_each(|(i, u)| total += i * TryInto::<usize>::try_into(*u).unwrap_or(0));
        height_map.generate(total);

        let mut generation = 1;
        while height_map.continuity_comparison() {
            if height_map.average_ground() == height_map.average_water() {
                height_map.invert();
            } else if height_map.average_water() == 0 {
                height_map.add_noise(generation);
                height_map.reduce_noise();
                height_map.scale();
            }

            height_map.evolve(generation);
            generation += 1;
        }
        height_map
    }

    fn display_new(seed: String) -> Self {  //displays each generation as it's generating
        let mut height_map = HeightMap::default();

        let mut total = 0;
        seed.as_bytes().iter().enumerate().for_each(|(i, u)| total += i * TryInto::<usize>::try_into(*u).unwrap_or(0));
        height_map.generate(total);

        let mut generation = 1;
        while height_map.continuity_comparison() {
            print!("\x1B[2J\x1B[1;1H");
            height_map.display();
            println!("generation: {}", generation);
            
            if height_map.average_ground() == height_map.average_water() {
                height_map.invert();
            } else if height_map.average_water() == 0 {
                height_map.add_noise(generation);
                height_map.reduce_noise();
                height_map.scale();
            }

            height_map.evolve(generation);
            generation += 1;
        }
        height_map
    }

    #[cfg(not(feature = "color"))]
    fn display(&self) {
        for vec in self.map.clone().into_iter() {
            for item in vec.into_iter() {
                let mut val;
                if item < self.water_level {
                    val = format!("{:02}", self.water_level - item);
                    
                } else {
                    val = format!("{:02}", item - self.water_level);
                }
                val.truncate(2);
                print!("{}", val);
            }
            println!("]");
        }
        println!("max: {:?} min: {:?}", self.max_point(), self.min_point());
        println!("cont: {:?} avg: {:?} lin: {} size: {}", self.count_continuity(), (self.average_ground(), self.average_water()), self.linear_ground(), self.size.0 * self.size.1);

    }

    #[cfg(feature = "color")]
    fn display(&self) {
        use coloriz::*;

        let max = self.max_point().0;
        let min = self.min_point().0;
        let ground_color_scale = 255 / (max - self.water_level);
        let water_color_scale = 255 / (self.water_level - min);

        for vec in self.map.clone().into_iter() {
            for item in vec.into_iter() {
                let mut val;
                if item < self.water_level {
                    val = format!("{:02}", self.water_level - item);
                    val.truncate(2);
                    let color = if item != min {(0, 0, ((self.water_level - item) * water_color_scale).try_into().unwrap_or(0))} else {(255, 255, 255)};
                    print!("{}", val.bg(color).invisible());
                    
                } else {
                    val = format!("{:02}", item - self.water_level);
                    val.truncate(2);
                    let color = if item != max {(0,((item - self.water_level) * ground_color_scale).try_into().unwrap_or(0), 0)} else {(255, 0, 0)};
                    print!("{}", val.bg(color).invisible());
                }
            }
            println!("]");
        }
        println!("max: {:?} min: {:?}", self.max_point(), self.min_point());
        println!("cont: {:?} avg: {:?} lin: {} size: {}", self.count_continuity(), (self.average_ground(), self.average_water()), self.linear_ground(), self.size.0 * self.size.1);

    }

    fn generate(&mut self, seed: usize) {
        self.seed = Some(seed);
        self.randomize(seed);
        self.hard_range();
        //self.invert();
        self.reduce_noise();
        self.evolve(seed);
        self.remove_edges();
    }

    fn evolve(&mut self, seed: usize) {
        self.add_noise(seed);
        self.clump();
        for _ in 1..((self.size.0 + self.size.1 + (self.max - self.min).try_into().unwrap_or(1)) / 128) {
            self.clump();
            self.fractal();
            self.slide();
        }
        self.migrate();
        self.fractal();
        self.brighten();
        self.saturate();
        self.scale();
    }

    fn slide(&mut self) {
        filter(&mut self.map,
        self.size.0,
        self.size.1,
        |m,_,i| if i >= m {i} else {-i},
        |m,i| m + i/16
        );
    }

    fn fractal(&mut self) {
        for vec in self.map.iter_mut() {
            for item in vec.iter_mut() {
                if *item > self.water_level && *item < (self.max - self.water_level) / 2 {
                    *item = -(*item - self.water_level);
                } else if *item < self.water_level && *item > (self.water_level - self.min) / 2 {
                    *item = -(self.water_level - *item);
                }
            }
        }
    }

    fn add_noise(&mut self, seed: usize) {
        let mut noise = self.map.clone();
        filter(&mut noise, self.size.0, self.size.1, |_,_,i| i, |_,i| HeightMap::last_prime((i.try_into().unwrap_or(seed)) % EFFICIENCYSCALE).try_into().unwrap_or(1));
        for i in 1..self.size.0 {
            for j in 1..self.size.1 {
                self.map[i][j] = (noise[i][j] * self.map[i][j] - noise[i][j] % (self.max - self.min) - (noise[i][j] / 2 * self.water_level)) / (seed.try_into().unwrap_or(1) % noise[i][j] + 1);
            }
        }
    }

    fn invert(&mut self) {
        for vec in self.map.iter_mut() {
            for item in vec.iter_mut() {
                *item *= -1;
            }
        }
    }

    fn brighten(&mut self) {
        for vec in self.map.iter_mut() {
            for item in vec.iter_mut() {
                if *item > self.water_level {
                    if *item > (self.max - self.water_level) / 2 {
                        *item = (*item + self.max) / 2;
                    } else {
                        *item = (*item + self.water_level) / 2;
                    }
                } else if *item > (self.water_level - self.min) / 2 {
                    *item = (*item + self.water_level) / 2;
                } else {
                    *item = (*item + self.min) / 2;
                }
            }
        }
    }

    fn saturate(&mut self) {
        filter(
            &mut self.map,
            self.size.0,
            self.size.1,
            |m,_,i| if i - m > 0 {i - m} else {m - i},
        |m,i|
            if i/8 > (self.max - self.min) / 2 {
                if m > self.water_level {
                    (m + self.max) / 2
                } else {
                    (m + self.min) / 2
                }
            } else {
                m
            }
        )
    }

    fn scale(&mut self) {
        let mut max = self.min;
        let mut min = self.max;

        for vec in self.map.clone().into_iter() {
            for item in vec.into_iter() {
                if item > max {
                    max = item;
                } else if item < min {
                    min = item;
                }
            }
        }

        let ground_scale = ((self.max - self.water_level), (max - self.water_level));
        let water_scale = ((self.water_level - self.min), (self.water_level - min));
        for vec in self.map.iter_mut() {
            for item in vec.iter_mut() {
                if *item > self.water_level {
                    *item = (*item * ground_scale.0 ) / if ground_scale.1 != 0 {ground_scale.1} else {1};
                } else {
                    *item = (*item * water_scale.0) / if water_scale.1 != 0 {water_scale.1} else {1};
                }
            }
        }
    }

    fn remove_edges(&mut self) {
        for i in 0..self.size.0 {
            self.map[i][0] = self.min;
            self.map[i][self.size.1-1] = self.min;
        }
        for i in 0..self.size.1 {
            self.map[0][i] = self.min;
            self.map[self.size.0-1][i] = self.min;
        }
    }

    fn reduce_noise(&mut self) {
        filter(
            &mut self.map,
            self.size.0,
            self.size.1,
            |m, _, i| i - m,
            |m, i| (m + (i / 8) + self.water_level) / 3);
    }

    fn clump(&mut self) {
        let mut ground = 0;
        let mut water = 0;
        for vec in get_filter(self.map.to_vec(), self.size.0, self.size.1, |_,_,i| if i > self.water_level {1} else {-1}).into_iter() {
            for item in vec.into_iter() {
                if item > self.water_level {
                    ground += 1;
                } else {
                    water += 1;
                }
            }
        }

        filter(
            &mut self.map,
            self.size.0,
            self.size.1,
            |_, _, i| if i > self.water_level {water} else {-ground},
            |m, i| if m > self.water_level {
                (m + i * (self.max - self.water_level) / (8 * (water + ground))) / 2
            } else {
                (m + i * (self.water_level - self.min) / (8 * (water + ground))) / 2
            });
    }

    fn migrate(&mut self) {
        filter(&mut self.map,
            self.size.0,
            self.size.1,
            |_, _, i| {
                if i > self.water_level {1} else {-1}
            },
            |m, i| if i > 0 {
                (i * (self.max - self.water_level)/8 + m) / 2
            } else {
                (i * (self.water_level - self.water_level)/8 + m) / 2
            }
        )
    }

    fn randomize(&mut self, seed: usize) {
        for i in 0..self.size.0 {
            let rand = self.random_value(HeightMap::last_prime(seed % EFFICIENCYSCALE) + HeightMap::next_prime((seed + i) % EFFICIENCYSCALE));
            if let Ok(r) = TryInto::<i32>::try_into(rand) {
                self.map[i][0] = r;
            }
        }
        for i in 0..self.size.1 {
            let rand = self.random_value(HeightMap::last_prime((seed + i) % EFFICIENCYSCALE) + HeightMap::next_prime(seed % EFFICIENCYSCALE));
            if let Ok(r) = TryInto::<i32>::try_into(rand) {
                self.map[0][i] = r;
            }
        }
        
        for i in 1..self.size.0 {
            for j in 1..self.size.1 {
                let rand = self.random_value(HeightMap::last_prime((seed * self.map[i-1][j].abs().try_into().unwrap_or(1)) % EFFICIENCYSCALE) + HeightMap::next_prime((seed * self.map[i][j-1].abs().try_into().unwrap_or(1)) % EFFICIENCYSCALE));
                if let Ok(r) = TryInto::<i32>::try_into(rand) {
                    self.map[i][j] = r;
                }
            }
        }
    }

    fn hard_range(&mut self) {  //ranges map without regard for height
        for vec in self.map.iter_mut() {
            for item in vec.iter_mut() {
                *item = (*item % (self.max - self.min)) + self.min;
            }
        }
    }

    fn count_ground(&self) -> usize {
        let mut ground = 0;
        for vec in self.map.clone().into_iter() {
            for item in vec.into_iter() {
                if item > self.water_level {
                    ground += 1;
                }
            }
        }
        ground
    }

    fn count_water(&self) -> usize {
        let mut water = 0;
        for vec in self.map.clone().into_iter() {
            for item in vec.into_iter() {
                if item < self.water_level {
                    water += 1;
                }
            }
        }
        water
    }

    fn average_ground (&self) -> usize {
        let mut total = 0;
        let scale = (self.max - self.water_level).try_into().unwrap_or(1);
        for vec in self.map.clone().into_iter() {
            for item in vec.into_iter() {
                if item > self.water_level {
                    total += (item - self.water_level).try_into().unwrap_or(0);
                }
            }
        }
        let ground = self.count_ground();
        if ground * scale != 0 {
            total / ground * scale
        } else {0}
    }

    fn average_water (&self) -> usize {
        let mut total = 0;
        let scale = (self.water_level - self.min).try_into().unwrap_or(1);
        for vec in self.map.clone().into_iter() {
            for item in vec.into_iter() {
                if item > self.water_level {
                    total += (self.water_level - item).try_into().unwrap_or(0);
                }
            }
        }
        let water = self.count_water();
        if water * scale != 0 {
            total / water * scale
        } else {0}
    }

    fn count_continuity(&self) -> (usize, usize) {    //returns num of tiles with similar neighbors
        let map = get_filter(self.map.clone(),
            self.size.0,
            self.size.1,
            |m,_,i| match (m > self.water_level, i > self.water_level) {
                (false, false) | (true, true) => 1,
                (false, true) | (true, false) => 0,
            });

        let mut ground_total = 0;
        let mut water_total = 0;
        for i in 0..self.size.0 {
            for j in 0..self.size.1 {
                match (self.map[i][j] > self.water_level, map[i][j].try_into().unwrap_or(0)) {
                    (true, total) => ground_total += total,
                    (false, total) => water_total += total,
                }
            }
        }
        (ground_total / 8, water_total / 8)
    }

    fn continuity_comparison(&self) -> bool {
        let continuity = self.count_continuity();
        3 * continuity.0 > 5 * continuity.1 ||
        continuity.1 > 2 * continuity.0 ||
        4 * self.linear_ground() < self.size.0 * self.size.1 ||
        //self.average_water() == 0 ||
        self.average_ground() == 0
    }

    fn linear_ground(&self) -> usize {
        let map = get_filter(self.map.clone(),
            self.size.0,
            self.size.1,
            |m,d,_| match (m > self.water_level, d[0] > self.water_level, d[1] > self.water_level, d[2] > self.water_level, d[3] > self.water_level, d[4] > self.water_level, d[5] > self.water_level, d[6] > self.water_level, d[7] > self.water_level, ) {
                (true, true, _, _, _, _, _, _, true) |
                (true, _, true, _, _, _, _, true, _) |
                (true, _, _, true, _, _, true, _, _) |
                (true, _, _, _, true, true, _, _, _) => 1,
                _ => -1,
            }
        );

        let mut linear_total = 0;
        for i in 0..self.size.0 {
            for j in 0..self.size.1 {
                if self.map[i][j] > self.water_level {
                    linear_total += map[i][j].try_into().unwrap_or(0);
                }
            }
        }
        linear_total / 8
    }

    fn random_value(&self, seed: usize) -> usize {
        let val = (seed * HeightMap::next_prime(seed)) % (seed * HeightMap::last_prime(seed));

        HeightMap::next_prime(val) * HeightMap::last_prime(val)
    }

    fn max_point(&self) -> (i32, usize, usize) {
        let mut max = (self.min, 0, 0);
        for i in 0..self.size.0 {
            for j in 0..self.size.1 {
                if max.0 < self.map[i][j] {
                    max = (self.map[i][j], i, j);
                }
            }
        }
        max
    }

    fn min_point(&self) -> (i32, usize, usize) {
        let mut min = (self.max, 0, 0);
        for i in 0..self.size.0 {
            for j in 0..self.size.1 {
                if min.0 > self.map[i][j] {
                    min = (self.map[i][j], i, j);
                }
            }
        }
        min
    }

    const fn next_prime(num: usize) -> usize {
        let mut prime = num+1;
        let mut i = 2;
        while i < prime / 2 {
            if prime % i != 0 {
                i += 1;
            } else {
                prime +=1;
                i = 2;
            }
        }
        return prime;
    }

    const fn last_prime(num: usize) -> usize {
        let mut prime = if num > 1 {num-1} else {3};
        let mut i = prime / 2;
        while i > 2 {
            if prime % i != 0 {
                i -= 1;
            } else {
                prime-=1;
                i = prime / 2;
            }
        }
        return prime;
    }
}

fn empty_matrix(height: usize, width: usize) -> Vec<Vec<i32>> {
    let mut matrix = Vec::new();
    for i in 0..width {
        matrix.push(Vec::new());
        for _ in 0..height {
            matrix[i].push(0);
        }
    }
    return matrix;
}

fn filter(
    matrix: &mut Vec<Vec<i32>>,
    height: usize,
    width: usize,
    filter_function: impl Fn(i32, [i32; 8], i32) -> i32,
    application_function: impl Fn(i32, i32) -> i32,
) {
    let filter = get_filter(matrix.to_vec(), height, width, filter_function);

    for i in 0..height {
        for j in 0..width {
            matrix[i][j] = application_function(matrix[i][j], filter[i][j]);
        }
    }
}

fn get_filter(
    matrix: Vec<Vec<i32>>,
    height: usize,
    width: usize,
    filter_function: impl Fn(i32, [i32; 8], i32) -> i32,
) -> Vec<Vec<i32>> {
    let mut filter = empty_matrix(height, width);

    for i in 1..height -1 {
        for j in 1..width -1 {
            let directions = [
                matrix[i-1][j-1], matrix[i][j-1], matrix[i+1][j-1],
                matrix[i-1][j], matrix[i+1][j],
                matrix[i-1][j+1], matrix[i][j+1], matrix[i+1][j+1]
            ];
            for item in directions {
                filter[i][j] += filter_function(matrix[i][j], directions, item);
            }
        }
    }
    filter
}