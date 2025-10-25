use nalgebra_glm::{Vec2, Vec3, Vec4, Mat4};
use minifb::{Key, Window, WindowOptions};
use std::f32::consts::PI;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;

// Estructura para representar un color
#[derive(Debug, Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    fn black() -> Self {
        Color::new(0, 0, 0)
    }

    fn to_u32(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    fn lerp(&self, other: &Color, t: f32) -> Color {
        Color::new(
            (self.r as f32 * (1.0 - t) + other.r as f32 * t) as u8,
            (self.g as f32 * (1.0 - t) + other.g as f32 * t) as u8,
            (self.b as f32 * (1.0 - t) + other.b as f32 * t) as u8,
        )
    }

    fn mul(&self, factor: f32) -> Color {
        Color::new(
            (self.r as f32 * factor).min(255.0) as u8,
            (self.g as f32 * factor).min(255.0) as u8,
            (self.b as f32 * factor).min(255.0) as u8,
        )
    }

    fn add(&self, other: &Color) -> Color {
        Color::new(
            (self.r as u16 + other.r as u16).min(255) as u8,
            (self.g as u16 + other.g as u16).min(255) as u8,
            (self.b as u16 + other.b as u16).min(255) as u8,
        )
    }
}

// Estructura para un fragmento
struct Fragment {
    position: Vec3,
    normal: Vec3,
    intensity: f32,
    vertex_position: Vec3,
}

// Estructura para vértices
struct Vertex {
    position: Vec3,
    normal: Vec3,
}

// Framebuffer
struct Framebuffer {
    buffer: Vec<u32>,
    zbuffer: Vec<f32>,
    width: usize,
    height: usize,
}

impl Framebuffer {
    fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            buffer: vec![0; width * height],
            zbuffer: vec![f32::INFINITY; width * height],
            width,
            height,
        }
    }

    fn clear(&mut self, color: Color) {
        let color_u32 = color.to_u32();
        for pixel in self.buffer.iter_mut() {
            *pixel = color_u32;
        }
        for z in self.zbuffer.iter_mut() {
            *z = f32::INFINITY;
        }
    }

    fn point(&mut self, x: usize, y: usize, color: Color, depth: f32) {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            if depth < self.zbuffer[index] {
                self.buffer[index] = color.to_u32();
                self.zbuffer[index] = depth;
            }
        }
    }
}

// Uniforms para pasar información al shader
struct Uniforms {
    model_matrix: Mat4,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    viewport_matrix: Mat4,
    time: f32,
    current_shader: u32,
}

// Funciones de ruido
fn random(x: f32, y: f32) -> f32 {
    let a = 12.9898;
    let b = 78.233;
    let c = 43758.5453;
    let dt = x * a + y * b;
    let sn = dt % PI;
    (sn.sin() * c).fract()
}

fn noise(x: f32, y: f32) -> f32 {
    let ix = x.floor();
    let iy = y.floor();
    let fx = x.fract();
    let fy = y.fract();

    let a = random(ix, iy);
    let b = random(ix + 1.0, iy);
    let c = random(ix, iy + 1.0);
    let d = random(ix + 1.0, iy + 1.0);

    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    let ab = a * (1.0 - ux) + b * ux;
    let cd = c * (1.0 - ux) + d * ux;
    ab * (1.0 - uy) + cd * uy
}

fn cellular_noise(x: f32, y: f32) -> f32 {
    let cell_x = x.floor();
    let cell_y = y.floor();
    let mut min_dist: f32 = 10.0;

    for i in -1..=1 {
        for j in -1..=1 {
            let neighbor_x = cell_x + i as f32;
            let neighbor_y = cell_y + j as f32;
            
            let point_x = neighbor_x + random(neighbor_x, neighbor_y);
            let point_y = neighbor_y + random(neighbor_y, neighbor_x);
            
            let dx = x - point_x;
            let dy = y - point_y;
            let dist = (dx * dx + dy * dy).sqrt();
            
            min_dist = min_dist.min(dist);
        }
    }

    min_dist
}

fn turbulence(x: f32, y: f32, size: f32) -> f32 {
    let mut value = 0.0;
    let mut scale = size;

    while scale > 1.0 {
        value += noise(x / scale, y / scale) * scale;
        scale /= 2.0;
    }

    value / size
}

// Shaders diferentes
fn sun_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let uv = Vec2::new(fragment.vertex_position.x, fragment.vertex_position.y);
    let time = uniforms.time;
    
    // CAPA 1: Gradiente radial del núcleo al borde
    let dist_from_center = (uv.x * uv.x + uv.y * uv.y).sqrt();
    let core_color = Color::new(255, 255, 220);
    let edge_color = Color::new(255, 120, 0);
    
    // CAPA 2: Manchas solares usando ruido
    let sunspot_noise = noise(uv.x * 5.0 + time * 0.1, uv.y * 5.0);
    let sunspots = if sunspot_noise < 0.35 { 0.6 } else { 1.0 };
    
    // CAPA 3: Protuberancias animadas
    let flare_noise = turbulence(
        uv.x * 3.0 + time * 0.3,
        uv.y * 3.0 + (time * 0.2).sin(),
        32.0
    );
    let flares = (flare_noise * 0.4 + 1.0).max(0.5);
    
    // CAPA 4: Emisión brillante (corona)
    let glow = (1.0 - dist_from_center * 0.9).max(0.0);
    let emission = glow.powf(2.5) * 0.8;
    
    // Combinar capas
    let base = core_color.lerp(&edge_color, dist_from_center);
    let with_spots = base.mul(sunspots);
    let with_flares = with_spots.mul(flares);
    let glow_color = Color::new(255, 200, 100);
    let final_color = with_flares.add(&glow_color.mul(emission));
    
    final_color
}

fn earth_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let uv = Vec2::new(
        fragment.vertex_position.x * 2.0,
        fragment.vertex_position.y * 2.0
    );
    
    // CAPA 1: Continentes y océanos
    let continent_noise = noise(uv.x * 3.0, uv.y * 3.0);
    let ocean_color = Color::new(20, 60, 160);
    let land_color = Color::new(80, 140, 60);
    let mountain_color = Color::new(100, 100, 90);
    let desert_color = Color::new(200, 180, 120);
    
    let mut base_color = if continent_noise > 0.6 {
        mountain_color
    } else if continent_noise > 0.4 {
        land_color
    } else if continent_noise > 0.38 {
        desert_color
    } else {
        ocean_color
    };
    
    // CAPA 2: Cráteres (solo en tierra)
    if continent_noise > 0.38 {
        let crater_noise = cellular_noise(uv.x * 10.0, uv.y * 10.0);
        if crater_noise < 0.08 {
            base_color = base_color.mul(0.5);
        }
    }
    
    // CAPA 3: Nubes
    let cloud_noise = noise(
        uv.x * 5.0 + uniforms.time * 0.1,
        uv.y * 5.0
    );
    let cloud_color = Color::new(255, 255, 255);
    let cloud_alpha = if cloud_noise > 0.55 {
        (cloud_noise - 0.55) * 2.0
    } else {
        0.0
    };
    
    // CAPA 4: Casquetes polares
    let latitude = fragment.vertex_position.y.abs();
    let ice_color = Color::new(240, 250, 255);
    if latitude > 0.75 {
        let ice_factor = ((latitude - 0.75) / 0.25).min(1.0);
        base_color = base_color.lerp(&ice_color, ice_factor);
    }
    
    // Aplicar nubes
    base_color = base_color.lerp(&cloud_color, cloud_alpha * 0.6);
    
    // Iluminación
    base_color.mul(fragment.intensity.max(0.2))
}

fn gas_giant_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let uv = Vec2::new(fragment.vertex_position.x, fragment.vertex_position.y);
    let time = uniforms.time;
    
    // CAPA 1: Bandas horizontales
    let latitude = uv.y;
    let band_freq = 8.0;
    let band_value = ((latitude + 1.0) * band_freq).sin();
    
    let band_color1 = Color::new(210, 180, 140);
    let band_color2 = Color::new(180, 140, 100);
    let band_color3 = Color::new(140, 100, 70);
    
    let mut base_color = if band_value > 0.3 {
        band_color1
    } else if band_value > -0.3 {
        band_color2
    } else {
        band_color3
    };
    
    // CAPA 2: Gran Mancha Roja
    let storm_x = 0.3;
    let storm_y = 0.2;
    let dist_to_storm = ((uv.x - storm_x).powi(2) + (uv.y - storm_y).powi(2)).sqrt();
    
    if dist_to_storm < 0.18 {
        let storm_color = Color::new(180, 60, 40);
        let storm_noise = noise(uv.x * 10.0, uv.y * 10.0);
        let storm_factor = (1.0 - (dist_to_storm / 0.18)) * 0.9;
        base_color = base_color.lerp(&storm_color, storm_factor * storm_noise);
    }
    
    // CAPA 3: Turbulencias
    let turb = turbulence(
        uv.x * 10.0 + time * 0.05,
        uv.y * 3.0,
        64.0
    );
    let turb_offset = (turb - 0.5) * 0.3;
    base_color = base_color.mul(1.0 + turb_offset);
    
    // CAPA 4: Remolinos en los límites de las bandas
    let swirl_noise = noise(uv.x * 20.0, uv.y * 5.0 + time * 0.1);
    if swirl_noise > 0.75 {
        let swirl_color = Color::new(200, 170, 130);
        base_color = base_color.lerp(&swirl_color, 0.4);
    }
    
    base_color.mul(fragment.intensity.max(0.15))
}

fn volcanic_planet_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let uv = Vec2::new(
        fragment.vertex_position.x * 2.0,
        fragment.vertex_position.y * 2.0
    );
    let time = uniforms.time;
    
    // CAPA 1: Base rocosa oscura
    let rock_color = Color::new(40, 30, 30);
    let lava_color = Color::new(255, 80, 0);
    
    // CAPA 2: Grietas de lava usando ruido celular
    let crack_noise = cellular_noise(uv.x * 15.0, uv.y * 15.0);
    let is_crack = crack_noise < 0.15;
    
    // CAPA 3: Pulsación de lava
    let pulse = ((time * 2.0).sin() * 0.5 + 0.5) * 0.5 + 0.5;
    
    // CAPA 4: Volcanes activos
    let volcano_noise = noise(uv.x * 3.0, uv.y * 3.0);
    let is_volcano = volcano_noise > 0.7;
    
    let mut base_color = if is_crack {
        lava_color.mul(pulse)
    } else if is_volcano {
        let glow = (time * 3.0 + volcano_noise * 10.0).sin() * 0.5 + 0.5;
        rock_color.lerp(&Color::new(255, 100, 0), glow * 0.7)
    } else {
        rock_color
    };
    
    // Emisión de luz desde las grietas
    if is_crack {
        base_color = base_color.add(&Color::new(100, 30, 0));
    }
    
    base_color.mul(fragment.intensity.max(0.3))
}

fn ice_planet_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Color {
    let uv = Vec2::new(
        fragment.vertex_position.x * 2.0,
        fragment.vertex_position.y * 2.0
    );
    
    // CAPA 1: Base de hielo
    let ice_color1 = Color::new(200, 220, 255);
    let ice_color2 = Color::new(150, 180, 230);
    
    // CAPA 2: Grietas en el hielo
    let crack_noise = cellular_noise(uv.x * 12.0, uv.y * 12.0);
    let is_crack = crack_noise < 0.1;
    
    // CAPA 3: Cristales de hielo
    let crystal_noise = noise(uv.x * 20.0, uv.y * 20.0);
    let is_crystal = crystal_noise > 0.75;
    
    // CAPA 4: Variación de superficie
    let surface_noise = turbulence(uv.x * 5.0, uv.y * 5.0, 32.0);
    
    let mut base_color = ice_color1.lerp(&ice_color2, surface_noise);
    
    if is_crack {
        base_color = base_color.mul(0.5);
    }
    
    if is_crystal {
        let crystal_color = Color::new(240, 250, 255);
        base_color = base_color.lerp(&crystal_color, 0.6);
    }
    
    base_color.mul(fragment.intensity.max(0.25))
}

fn desert_planet_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let uv = Vec2::new(
        fragment.vertex_position.x * 2.0,
        fragment.vertex_position.y * 2.0
    );
    let time = uniforms.time;
    
    // CAPA 1: Colores de arena
    let sand_color1 = Color::new(220, 180, 120);
    let sand_color2 = Color::new(200, 150, 90);
    let sand_color3 = Color::new(180, 130, 70);
    
    // CAPA 2: Dunas (ondulaciones)
    let dune_pattern = ((uv.x * 10.0).sin() + (uv.y * 10.0).cos()) * 0.5 + 0.5;
    
    let mut base_color = if dune_pattern > 0.6 {
        sand_color1
    } else if dune_pattern > 0.3 {
        sand_color2
    } else {
        sand_color3
    };
    
    // CAPA 3: Tormentas de arena
    let storm_noise = turbulence(
        uv.x * 5.0 + time * 0.2,
        uv.y * 5.0,
        32.0
    );
    base_color = base_color.mul(0.7 + storm_noise * 0.3);
    
    // CAPA 4: Formaciones rocosas
    let rock_noise = cellular_noise(uv.x * 8.0, uv.y * 8.0);
    if rock_noise < 0.05 {
        let rock_color = Color::new(100, 80, 60);
        base_color = base_color.lerp(&rock_color, 0.7);
    }
    
    base_color.mul(fragment.intensity.max(0.2))
}

fn moon_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Color {
    let uv = Vec2::new(
        fragment.vertex_position.x * 2.0,
        fragment.vertex_position.y * 2.0
    );
    
    // CAPA 1: Base gris lunar
    let base_color = Color::new(140, 140, 145);
    let dark_color = Color::new(80, 80, 85);
    
    // CAPA 2: Cráteres densos
    let crater_noise = cellular_noise(uv.x * 20.0, uv.y * 20.0);
    let crater_factor = if crater_noise < 0.15 { 0.6 } else { 1.0 };
    
    // CAPA 3: Mares lunares
    let mare_noise = noise(uv.x * 2.0, uv.y * 2.0);
    let is_mare = mare_noise < 0.3;
    
    // CAPA 4: Cráteres grandes
    let big_crater = cellular_noise(uv.x * 5.0, uv.y * 5.0);
    let is_big_crater = big_crater < 0.1;
    
    let mut final_color = if is_mare {
        dark_color
    } else {
        base_color
    };
    
    final_color = final_color.mul(crater_factor);
    
    if is_big_crater {
        final_color = final_color.mul(0.5);
    }
    
    final_color.mul(fragment.intensity.max(0.15))
}

fn ring_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Color {
    let distance = (fragment.vertex_position.x.powi(2) + fragment.vertex_position.z.powi(2)).sqrt();
    
    // Anillos concéntricos con gaps
    let ring_pattern = (distance * 30.0).sin();
    
    // División de Cassini (gap)
    let in_gap = distance > 0.35 && distance < 0.42;
    
    if in_gap {
        return Color::new(0, 0, 0); // Transparente
    }
    
    // Colores de los anillos
    let ring_color1 = Color::new(180, 170, 150);
    let ring_color2 = Color::new(150, 140, 120);
    
    let base = if ring_pattern > 0.0 {
        ring_color1
    } else {
        ring_color2
    };
    
    // Variación con ruido
    let variation = noise(
        fragment.vertex_position.x * 80.0,
        fragment.vertex_position.z * 80.0
    );
    
    base.mul(0.7 + variation * 0.3)
}

// Función principal del fragment shader
fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    match uniforms.current_shader {
        0 => sun_shader(fragment, uniforms),
        1 => earth_shader(fragment, uniforms),
        2 => gas_giant_shader(fragment, uniforms),
        3 => volcanic_planet_shader(fragment, uniforms),
        4 => ice_planet_shader(fragment, uniforms),
        5 => desert_planet_shader(fragment, uniforms),
        6 => moon_shader(fragment, uniforms),
        7 => ring_shader(fragment, uniforms),
        _ => Color::new(255, 0, 255), // Magenta para errores
    }
}

// Vertex shader
fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vec4 {
    let position = Vec4::new(
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        1.0
    );
    
    uniforms.projection_matrix * uniforms.view_matrix * uniforms.model_matrix * position
}

// Crear una esfera CORREGIDA
fn create_sphere(radius: f32, rings: u32, sectors: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    let r = 1.0 / (rings - 1) as f32;
    let s = 1.0 / (sectors - 1) as f32;
    
    for ring in 0..rings {
        for sector in 0..sectors {
            let theta = PI * ring as f32 * r;
            let phi = 2.0 * PI * sector as f32 * s;
            
            let x = theta.sin() * phi.cos();
            let y = theta.cos();
            let z = theta.sin() * phi.sin();
            
            vertices.push(Vertex {
                position: Vec3::new(x * radius, y * radius, z * radius),
                normal: Vec3::new(x, y, z),
            });
        }
    }
    
    for ring in 0..rings - 1 {
        for sector in 0..sectors - 1 {
            let current = ring * sectors + sector;
            let next = current + sectors;
            
            indices.push(current);
            indices.push(next);
            indices.push(current + 1);
            
            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }
    
    (vertices, indices)
}

// Crear un anillo (disco plano)
fn create_ring(inner_radius: f32, outer_radius: f32, segments: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    for i in 0..=segments {
        let angle = 2.0 * PI * i as f32 / segments as f32;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        // Vértice interior
        vertices.push(Vertex {
            position: Vec3::new(cos_a * inner_radius, 0.0, sin_a * inner_radius),
            normal: Vec3::new(0.0, 1.0, 0.0),
        });
        
        // Vértice exterior
        vertices.push(Vertex {
            position: Vec3::new(cos_a * outer_radius, 0.0, sin_a * outer_radius),
            normal: Vec3::new(0.0, 1.0, 0.0),
        });
    }
    
    for i in 0..segments {
        let base = i * 2;
        
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        
        indices.push(base + 1);
        indices.push(base + 3);
        indices.push(base + 2);
    }
    
    (vertices, indices)
}

// Renderizar un triángulo
fn triangle(v1: &Vec4, v2: &Vec4, v3: &Vec4, 
            n1: &Vec3, n2: &Vec3, n3: &Vec3,
            vp1: &Vec3, vp2: &Vec3, vp3: &Vec3,
            framebuffer: &mut Framebuffer, 
            uniforms: &Uniforms) {
    
    let (a, b, c) = (v1, v2, v3);
    
    let min_x = a.x.min(b.x).min(c.x).floor() as i32;
    let min_y = a.y.min(b.y).min(c.y).floor() as i32;
    let max_x = a.x.max(b.x).max(c.x).ceil() as i32;
    let max_y = a.y.max(b.y).max(c.y).ceil() as i32;
    
    let min_x = min_x.max(0);
    let min_y = min_y.max(0);
    let max_x = max_x.min(framebuffer.width as i32 - 1);
    let max_y = max_y.min(framebuffer.height as i32 - 1);
    
    let light_dir = Vec3::new(0.0, 0.0, 1.0);
    
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);
            
            let (w1, w2, w3) = barycentric(&Vec3::new(a.x, a.y, 0.0),
                                           &Vec3::new(b.x, b.y, 0.0),
                                           &Vec3::new(c.x, c.y, 0.0),
                                           &point);
            
            if w1 >= 0.0 && w2 >= 0.0 && w3 >= 0.0 {
                let depth = w1 * a.z + w2 * b.z + w3 * c.z;
                
                let normal = (n1 * w1 + n2 * w2 + n3 * w3).normalize();
                let vertex_pos = vp1 * w1 + vp2 * w2 + vp3 * w3;
                
                let intensity = normal.dot(&light_dir).max(0.0);
                
                let fragment = Fragment {
                    position: Vec3::new(x as f32, y as f32, depth),
                    normal,
                    intensity,
                    vertex_position: vertex_pos,
                };
                
                let color = fragment_shader(&fragment, uniforms);
                framebuffer.point(x as usize, y as usize, color, depth);
            }
        }
    }
}

fn barycentric(a: &Vec3, b: &Vec3, c: &Vec3, p: &Vec3) -> (f32, f32, f32) {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    
    let d00 = v0.dot(&v0);
    let d01 = v0.dot(&v1);
    let d11 = v1.dot(&v1);
    let d20 = v2.dot(&v0);
    let d21 = v2.dot(&v1);
    
    let denom = d00 * d11 - d01 * d01;
    
    if denom.abs() < 1e-6 {
        return (0.0, 0.0, 0.0);
    }
    
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    
    (u, v, w)
}

fn render(framebuffer: &mut Framebuffer, uniforms: &Uniforms, 
          vertices: &[Vertex], indices: &[u32]) {
    
    let mut transformed_vertices = Vec::new();
    
    for vertex in vertices {
        let pos = vertex_shader(vertex, uniforms);
        transformed_vertices.push((pos, vertex.normal, vertex.position));
    }
    
    for i in (0..indices.len()).step_by(3) {
        let i1 = indices[i] as usize;
        let i2 = indices[i + 1] as usize;
        let i3 = indices[i + 2] as usize;
        
        let (v1, n1, vp1) = &transformed_vertices[i1];
        let (v2, n2, vp2) = &transformed_vertices[i2];
        let (v3, n3, vp3) = &transformed_vertices[i3];
        
        let mut v1_ndc = *v1;
        let mut v2_ndc = *v2;
        let mut v3_ndc = *v3;
        
        if v1_ndc.w != 0.0 {
            v1_ndc.x /= v1_ndc.w;
            v1_ndc.y /= v1_ndc.w;
            v1_ndc.z /= v1_ndc.w;
        }
        if v2_ndc.w != 0.0 {
            v2_ndc.x /= v2_ndc.w;
            v2_ndc.y /= v2_ndc.w;
            v2_ndc.z /= v2_ndc.w;
        }
        if v3_ndc.w != 0.0 {
            v3_ndc.x /= v3_ndc.w;
            v3_ndc.y /= v3_ndc.w;
            v3_ndc.z /= v3_ndc.w;
        }
        
        let v1_screen = uniforms.viewport_matrix * v1_ndc;
        let v2_screen = uniforms.viewport_matrix * v2_ndc;
        let v3_screen = uniforms.viewport_matrix * v3_ndc;
        
        triangle(&v1_screen, &v2_screen, &v3_screen,
                n1, n2, n3,
                vp1, vp2, vp3,
                framebuffer, uniforms);
    }
}

fn create_model_matrix(translation: Vec3, rotation: Vec3, scale: f32) -> Mat4 {
    let translation_matrix = Mat4::new_translation(&translation);
    
    let rotation_x = Mat4::from_euler_angles(rotation.x, 0.0, 0.0);
    let rotation_y = Mat4::from_euler_angles(0.0, rotation.y, 0.0);
    let rotation_z = Mat4::from_euler_angles(0.0, 0.0, rotation.z);
    
    let rotation_matrix = rotation_z * rotation_y * rotation_x;
    
    let scale_matrix = Mat4::new_scaling(scale);
    
    translation_matrix * rotation_matrix * scale_matrix
}

fn create_view_matrix(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    nalgebra_glm::look_at(&eye, &center, &up)
}

fn create_perspective_matrix(fov: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    nalgebra_glm::perspective(aspect, fov, near, far)
}

fn create_viewport_matrix(width: f32, height: f32) -> Mat4 {
    Mat4::new(
        width / 2.0, 0.0, 0.0, width / 2.0,
        0.0, -height / 2.0, 0.0, height / 2.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    )
}

fn main() {
    let mut window = Window::new(
        "Sistema Solar - Shaders Procedurales",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    
    // Crear geometría - TAMAÑO AUMENTADO
    let (sphere_vertices, sphere_indices) = create_sphere(1.5, 30, 30);
    let (ring_vertices, ring_indices) = create_ring(1.95, 2.7, 60);
    
    let mut time = 0.0;
    let mut current_shader = 0u32;
    let mut rotation = 0.0f32;
    let mut paused = false;
    
    println!("=== CONTROLES ===");
    println!("1-7: Cambiar shader (Sol, Tierra, Júpiter, Volcán, Hielo, Desierto, Luna)");
    println!("8: Júpiter con anillos");
    println!("ESPACIO: Pausar/Reanudar rotación");
    println!("R: Resetear rotación");
    println!("ESC: Salir");
    println!("\nShader actual: 0 - Sol");
    
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Input handling
        if window.is_key_pressed(Key::Key1, minifb::KeyRepeat::No) {
            current_shader = 0;
            println!("Shader: 0 - Sol");
        }
        if window.is_key_pressed(Key::Key2, minifb::KeyRepeat::No) {
            current_shader = 1;
            println!("Shader: 1 - Tierra");
        }
        if window.is_key_pressed(Key::Key3, minifb::KeyRepeat::No) {
            current_shader = 2;
            println!("Shader: 2 - Júpiter (Gigante Gaseoso)");
        }
        if window.is_key_pressed(Key::Key4, minifb::KeyRepeat::No) {
            current_shader = 3;
            println!("Shader: 3 - Planeta Volcánico");
        }
        if window.is_key_pressed(Key::Key5, minifb::KeyRepeat::No) {
            current_shader = 4;
            println!("Shader: 4 - Planeta Helado");
        }
        if window.is_key_pressed(Key::Key6, minifb::KeyRepeat::No) {
            current_shader = 5;
            println!("Shader: 5 - Planeta Desierto");
        }
        if window.is_key_pressed(Key::Key7, minifb::KeyRepeat::No) {
            current_shader = 6;
            println!("Shader: 6 - Luna");
        }
        if window.is_key_pressed(Key::Key8, minifb::KeyRepeat::No) {
            println!("Modo: Júpiter con Anillos");
        }
        if window.is_key_pressed(Key::Space, minifb::KeyRepeat::No) {
            paused = !paused;
            println!("Rotación: {}", if paused { "PAUSADA" } else { "ACTIVA" });
        }
        if window.is_key_pressed(Key::R, minifb::KeyRepeat::No) {
            rotation = 0.0;
            time = 0.0;
            println!("Rotación reseteada");
        }
        
        framebuffer.clear(Color::new(0, 0, 10));
        
        if !paused {
            time += 0.016;
            rotation += 0.01;
        }
        
        let model_matrix = create_model_matrix(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, rotation, 0.0),
            1.0
        );
        
        // CÁMARA MÁS CERCA
        let view_matrix = create_view_matrix(
            Vec3::new(0.0, 0.0, 3.5),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0)
        );
        
        let projection_matrix = create_perspective_matrix(
            45.0 * PI / 180.0,
            WIDTH as f32 / HEIGHT as f32,
            0.1,
            100.0
        );
        
        let viewport_matrix = create_viewport_matrix(WIDTH as f32, HEIGHT as f32);
        
        let uniforms = Uniforms {
            model_matrix,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            time,
            current_shader,
        };
        
        // Renderizar planeta
        render(&mut framebuffer, &uniforms, &sphere_vertices, &sphere_indices);
        
        // Renderizar anillos si es tecla 8
        if window.is_key_down(Key::Key8) {
            let ring_uniforms = Uniforms {
                model_matrix,
                view_matrix,
                projection_matrix,
                viewport_matrix,
                time,
                current_shader: 7, // Ring shader
            };
            render(&mut framebuffer, &ring_uniforms, &ring_vertices, &ring_indices);
        }
        
        window
            .update_with_buffer(&framebuffer.buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}