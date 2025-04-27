use rand::Rng;
use rayon::prelude::*;
use std::{collections::VecDeque, f32};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

const WIDTH: u32 = 1000;
const HEIGHT: u32 = 800;

const WIDTH_F32: f32 = WIDTH as f32;
const HEIGHT_F32: f32 = HEIGHT as f32;

const SMOOTHING_RAD: f32 = 2.0;

#[derive(Clone, Copy)]
pub struct House {
    pub pos: Vec2,
    pub group: u8,
    pub friends: u8,
    pub enemies: u8,
    pub nearest: [(usize, f32); 2],
}

#[derive(Resource, Clone)]
pub struct HouseVec(Option<Vec<House>>);


#[derive(Resource)]
pub struct BorderPixels {
    pixels: Box<[[(u16, u16); HEIGHT as usize]; WIDTH as usize]>,
}

impl BorderPixels {
    pub fn new() -> Self {
        Self {
            pixels: Box::new([[(1001, 1001); HEIGHT as usize]; WIDTH as usize]),
        }
    }
}

#[derive(Resource)]
pub struct ImageHandleRes(AssetId<Image>);

#[derive(Component)]
pub struct Ball;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "coordinate borders".to_string(), // Set the window title
                resolution: (WIDTH_F32, HEIGHT_F32).into(), // Set width and height
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_borders, get_border_points))
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0))) // Set background color
        .insert_resource(HouseVec(None))
        .insert_resource(BorderPixels::new())
        .run();
}

fn get_border_points(
    pixels: ResMut<BorderPixels>,
    mut images: ResMut<Assets<Image>>,
    keys: Res<ButtonInput<KeyCode>>,
    id: Res<ImageHandleRes>,
) {

    if !keys.just_pressed(KeyCode::ArrowRight) {
        return;
    }

    let mut start_pixel = None;

    
    for (x, row) in pixels.pixels.iter().enumerate() {
        for (y, &pixel) in row.iter().enumerate() {
            if pixel != (1001, 1001) {
                start_pixel = Some((x, y, pixel));
                break;
            }
        }
        if start_pixel.is_some() {
            break;
        }
    }
    
    if start_pixel.is_none() {
        println!("No non-default pixels found.");
        return;
    }
    
    let (x, y, nearest) = start_pixel.unwrap();
    println!("First non-default pixel found at ({}, {}): {:?}", x, y, nearest);

    // wether the pixels have been visited yet
    let mut visited = [[false; HEIGHT as usize]; WIDTH as usize];

    // the queue that has the pixels x and y, 
    // along with the previous pixels nearest
    let mut queue: VecDeque<(usize, usize, (u16, u16))> = VecDeque::new();

    // x and y coordinate of the vertex
    let mut verteces: Vec<(usize, usize)> = Vec::new();

    queue.push_back((x, y, nearest));

    while let Some(pixel) = queue.pop_front() {
        let (px, py, nearest) = pixel;
    
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = px as isize + dx;
            let ny = py as isize + dy;
    
            if nx >= 0 && nx < WIDTH as isize && ny >= 0 && ny < HEIGHT as isize {
                let nx = nx as usize;
                let ny = ny as usize;
    
                if !visited[nx][ny] && !(pixels.pixels[nx][ny] == (1001, 1001)) {
                    visited[nx][ny] = true; // Mark as visited before adding to the queue
    
                    if pixels.pixels[nx][ny] != (nearest.0, nearest.1)
                        && pixels.pixels[nx][ny] != (nearest.1, nearest.0)
                    {
                        verteces.push((nx, ny)); // Push the correct vertex
                    }
    
                    queue.push_back((nx, ny, pixels.pixels[nx][ny]));
                }
            }
        }
    }

    if let Some(image) = images.get_mut(&Handle::Weak(id.0)) {
        for (vx, vy) in verteces {
            for dx in 0..5 {
                for dy in 0..5 {
                    let nx = vx + dx;
                    let ny = vy + dy;
    
                    if nx < WIDTH as usize && ny < HEIGHT as usize {
                        let start = (nx + (ny * WIDTH as usize)) * 4;
                        image.data[start..start + 4].copy_from_slice(&[255, 0, 0, 255]); // Red color
                    }
                }
            }
        }
    }
}

pub struct NearestHouses {
    nearest: Vec<(usize, f32)>
}

impl NearestHouses {
    pub fn new() -> Self {
        return Self { nearest: Vec::new() };
    }
    
    pub fn add(&mut self, point: (usize, f32)) {
   
        if self.nearest.is_empty() {
            self.nearest.push(point);
            return;
        }


        // sort into ascending distance
        self.nearest.push(point);
        self.nearest.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let nearest_point = self.nearest[0];

        for i in 1..self.nearest.len() {
            if self.nearest[i].1 > nearest_point.1 + SMOOTHING_RAD {
                while self.nearest.len() > i {
                    self.nearest.pop();
                }

                break;
            }
        }
    }

    pub fn is_border (&self, points: &Vec<House>) -> bool {

        if self.nearest.len() <= 1 {
            return false;
        };

        let last_group = points[self.nearest[0].0].group;

        for i in 1..self.nearest.len() {
            if points[self.nearest[i].0].group != last_group {
                return true;
            }
        }

        return false

    }

    pub fn get_houses(&self, points: &Vec<House>) -> Option<[(usize, f32); 2]> {
        if self.nearest.len() <= 1 {
            return None;
        };

        let last = self.nearest[0];
        let last_group = points[last.0].group;

        for i in 1..self.nearest.len() {
            if points[self.nearest[i].0].group != last_group {
                return Some([last, self.nearest[i]]);
            }
        }

        return None
    }
}

fn draw_borders(
    keys: Res<ButtonInput<KeyCode>>,
    house_vec: Res<HouseVec>,
    id: Res<ImageHandleRes>,
    mut images: ResMut<Assets<Image>>,
    mut pixels: ResMut<BorderPixels>
) {
    if keys.just_pressed(KeyCode::Space) {
        if let Some(vec) = &house_vec.0 {
            let handle = Handle::Weak(id.0);

            if let Some(image) = images.get_mut(&handle) {
                let image_data = std::sync::Arc::new(std::sync::Mutex::new(&mut image.data));
                let pixels_data = std::sync::Arc::new(std::sync::Mutex::new(&mut pixels.pixels));

                // Use rayon's parallel iterator to process rows in parallel
                (0..HEIGHT).into_par_iter().for_each(|y| {
                    for x in 0..WIDTH {

                        let mut nearest: NearestHouses = NearestHouses::new();
                        let x = x as f32;
                        let y = y as f32;

                        for (i, house) in vec.iter().enumerate() {
                            let dist = house.pos.distance(Vec2 { x, y });

                            nearest.add((i, dist));
                        }

                        if nearest.is_border(&vec)
                        {
                            let nx = x as usize;
                            let ny = y as usize;

                            let nearest_houses = nearest.get_houses(&vec).unwrap();

                            let mut pixels_data = pixels_data.lock().unwrap();
                            pixels_data[nx][ny] = (nearest_houses[0].0 as u16, nearest_houses[1].0 as u16);

                            if nx < WIDTH as usize && ny < HEIGHT as usize {
                                let start = (nx + (ny * WIDTH as usize)) * 4;

                                // Merge the colors by averaging their RGBA values
                                let color1 = pick_colour(vec[nearest_houses[0].0].group);
                                let color2 = pick_colour(vec[nearest_houses[1].0].group);

                                let merged_color = [
                                    ((color1[0] as u16 + color2[0] as u16) / 2) as u8,
                                    ((color1[1] as u16 + color2[1] as u16) / 2) as u8,
                                    ((color1[2] as u16 + color2[2] as u16) / 2) as u8,
                                    255,
                                ];

                                let mut data = image_data.lock().unwrap();
                                data[start..start + 4].copy_from_slice(&merged_color);
                            }
                        }
                    }
                });
            }
        }

        println!("finished")
    }
}

fn setup(mut commands: Commands, mut vec: ResMut<HouseVec>, mut images: ResMut<Assets<Image>>) {
    let mut rng = rand::rng();

    vec.0 = Some(Vec::new());

    for _ in 0..750 {
        let x: f32 = rng.random_range(0.0..WIDTH_F32);
        let y: f32 = rng.random_range(0.0..HEIGHT_F32);

        let mut group = 0;

        if (x - 50.0).powi(2) + (y - 50.0).powi(2) > 200.0_f32.powi(2) {
            group = 1;
        }

        if (x - 400.0).powi(2) + (y - 400.0).powi(2) < 350.0_f32.powi(2) {
            group = 2;
        }

        if (x - 750.0).powi(2) + (y).powi(2) < 200.0_f32.powi(2) {
            group = 3;
        }

        if ((x - 750.0) / 2.0).powi(2) + (y - 300.0).powi(2) < 200.0_f32.powi(2) {
            group = 4;
        }

        if let Some(ref mut houses) = vec.0 {
            houses.push(House {
                pos: Vec2::new(x, y),
                group,
                friends: 0,
                enemies: 0,
                nearest: [(0, f32::INFINITY), (0, f32::INFINITY)],
            });
        }
    }

    commands.spawn(Camera2d);

    let size = Extent3d {
        width: WIDTH,
        height: HEIGHT,
        depth_or_array_layers: 1,
    };
    let mut image: Image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::all(),
    );

    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    for coord in vec.0.clone().unwrap() {
        let col = pick_colour(coord.group);

        let x = coord.pos.x as usize;
        let y = coord.pos.y as usize;

        for dx in 0..10 {
            for dy in 0..10 {
                let nx = x + dx;
                let ny = y + dy;

                if nx < WIDTH as usize && ny < HEIGHT as usize {
                    let start = (nx + (ny * 1000)) * 4;
                    image.data[start..start + 4].copy_from_slice(&col);
                }
            }
        }
    }

    let image = images.add(image);
    let id = image.id();

    commands.insert_resource(ImageHandleRes(id));

    commands.spawn((
        Sprite::from_image(image),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

fn pick_colour(group: u8) -> [u8; 4] {
    match group {
        1 => [255, 0, 0, 255],
        2 => [0, 255, 0, 255],
        3 => [0, 0, 255, 255],
        4 => [175, 75, 25, 255],
        _ => [255, 255, 255, 255],
    }
}
