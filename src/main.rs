use std::ops::Mul;

use raylib::prelude::*;

fn main() {
    let width = 720;
    let height = 720;

    let (mut rl, thread) = raylib::init()
        .size(width, height)
        .title("oscae-raytracer")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

        let mut color_grid: Vec<Vec<Color>> = vec![vec![Color::BLACK; width as usize]; height as usize];

    let mut draw_process = 0;
    let draw_batch = 720 * 720;

    let mut scene = Scene {
        objects: vec![
            Box::new(Sphere {
                center: Vector3::new(0.0, 0.0, 5.0),
                radius: 1.0,
                color: Color::RED,
            }),
            Box::new(Sphere {
                center: Vector3::new(3.0, -1.0, 20.0),
                radius: 10.0,
                color: Color::BLUE,
            }),
            Box::new(Sphere {
                center: Vector3::new(-4.0, 4.0, 10.0),
                radius: 3.0,
                color: Color::new(0xE8, 0x3D, 0x84, 0xFF),
            }),
            Box::new(Plane {
                point: Vector3::new(0.0, -1.0, 0.0),
                normal: Vector3::new(0.0, 1.0, 0.0),
                color: Color::GREEN,
            }),
        ],
        point_lights: vec![
                Vector3::new(-2.0, 0.0, 5.0),
                //Vector3::new(-5.0, 5.0, 3.0),
            ],
        directional_light: Vector3::new(-2.0, -2.0, 1.0),
    };

    let mut velocity = Vector3::zero();
    let mut position = Vector3::new(0.0, 0.0, 5.0);

    while !rl.window_should_close() {
        // Update
        
        position += velocity * rl.get_frame_time();
        if position.y < 0.0 {
            position.y = 0.0;
            velocity.y = -velocity.y * 0.8;
            if velocity.y.abs() < 1.0 {
                velocity.y = 0.0;
                position.y = 0.0;
            }
        } else if position.y > 0.0 {
            velocity -= Vector3::new(0.0, 2.0, 0.00) * rl.get_frame_time();
        }

        scene.objects[0].set_position(position);

        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            
            velocity.y = 5.0;
        }
        
        for _ in 0..draw_batch {
            if draw_process >= width * height {
                draw_process = 0;
                break;
            }

            // raytracer
            let x = draw_process % width;
            let y = draw_process / width;

            
            color_grid[y as usize][x as usize] = raytracer(&scene, Vector3::zero(), ray_from_pixel(x, y, width, height));

            draw_process += 1;
        }

        // Draw
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);

        for y in 0..height {
            for x in 0..width {
                d.draw_pixel(x, y, color_grid[y as usize][x as usize]);
            }
        }
    }       
}

fn ray_from_pixel(x: i32, y: i32, width: i32, height: i32) -> Vector3 {
    // fov is 90 degrees
    let x = (x - width / 2) as f32 + 0.5;
    let y = (height / 2 - y) as f32 - 0.5;
    let z = width as f32 / 2.0;

    let mut v = Vector3::new(x, y, z);
    v.normalize();
    v
}

fn raytracer(scene: &Scene, origin: Vector3, direction: Vector3) -> Color {
    let mut color = Color::BLACK;
    let mut min_t = f32::INFINITY;
    let mut min_point = Vector3::zero();
    let mut min_normal = Vector3::zero();

    for object in scene.objects.iter() {
        if let Some((t, point)) = object.intersection(origin, direction) {
            if t < min_t {
                min_t = t;
                min_point = point;
                min_normal = object.normal(point);
                color = object.color();
            }
        }
    }

    // shadow
    let mut light_intensity: f32 = 0.0;
    // point lights
    for &light in &scene.point_lights {
        let mut light_direction = light - min_point;
        light_direction.normalize();
        let shadow_origin = min_point + light_direction * 0.001;
        let distance_to_light = (light - min_point).length();
        let mut in_shadow = false;
        for object in scene.objects.iter() {
            if let Some((t, _)) = object.intersection(shadow_origin, light_direction) {
                if t < distance_to_light {
                    in_shadow = true;
                    break;
                }
            }
        }
        if !in_shadow {
            light_intensity += min_normal.dot(light_direction).max(0.0);
        }
    }

    // drirectional light
    let mut light_direction = -scene.directional_light;
    light_direction.normalize();
    let shadow_origin = min_point + light_direction * 0.001;

    let mut in_shadow = false;
    for object in scene.objects.iter() {
        if object.intersection(shadow_origin, light_direction).is_some() {
            in_shadow = true;
            break;
        }
    }
    if !in_shadow {
        light_intensity += min_normal.dot(light_direction).max(0.0);
    }
    
    color = darken(color, light_intensity.max(0.1));

    color
}

fn darken(color: Color, factor: f32) -> Color {
    Color {
        r: (color.r as f32 * factor).min(255.0).max(0.0) as u8,
        g: (color.g as f32 * factor).min(255.0).max(0.0) as u8,
        b: (color.b as f32 * factor).min(255.0).max(0.0) as u8,
        a: color.a
    }
}

struct Scene {
    objects: Vec<Box<dyn Object>>,
    point_lights: Vec<Vector3>,
    directional_light: Vector3,
}

trait Object {
    fn intersection(&self, ray_origin: Vector3, ray_direction: Vector3) -> Option<(f32, Vector3)>;
    fn color(&self) -> Color;
    fn normal(&self, point: Vector3) -> Vector3;
    fn set_position(&mut self, position: Vector3);
}

struct Sphere {
    center: Vector3,
    radius: f32,
    color: Color,
}

impl Object for Sphere {
    fn intersection(&self, ray_origin: Vector3, ray_direction: Vector3) -> Option<(f32, Vector3)> {
        // t = (-b ± √(b² - 4ac)) / 2a
        let oc = ray_origin - self.center;
        let a = ray_direction.dot(ray_direction);
        let b = 2.0 * oc.dot(ray_direction);
        let c = oc.dot(oc) - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            None
        } else {
            let discriminant_sqrt = discriminant.sqrt();
            let t1 = (-b - discriminant_sqrt) / (2.0 * a);
            let t2 = (-b + discriminant_sqrt) / (2.0 * a);
            
            let t = if t1 >= 0.0 {
                t1
            } else if t2 >= 0.0 {
                t2
            } else {
                return None;
            };
            
            let point = ray_origin + ray_direction * t;
            Some((t, point))
        }
    }

    fn color(&self) -> Color {
        self.color
    }

    fn normal(&self, point: Vector3) -> Vector3 {
        (point - self.center) / self.radius
    }

    fn set_position(&mut self, position: Vector3) {
        self.center = position;
    }
}

struct Plane {
    point: Vector3,
    normal: Vector3,
    color: Color,
}

impl Object for Plane {
    fn intersection(&self, ray_origin: Vector3, ray_direction: Vector3) -> Option<(f32, Vector3)> {
        let denom = self.normal.dot(ray_direction);
        if denom.abs() > 1e-6 {
            let t = (self.point - ray_origin).dot(self.normal) / denom;
            if t >= 0.0 {
                let point = ray_origin + ray_direction * t;
                return Some((t, point));
            }
        }
        None
    }

    fn color(&self) -> Color {
        self.color
    }

    fn normal(&self, _point: Vector3) -> Vector3 {
        self.normal
    }

    fn set_position(&mut self, position: Vector3) {
        self.point = position;
    }
}