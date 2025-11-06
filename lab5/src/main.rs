// =============================================================================
// Sistema Solar Interactivo con Renderizado GPU - VERSIÓN UNIFICADA
// Autor: Pablo Cabrera
// Carné: 231156
// Descripción: Todos los módulos concentrados en un solo archivo
// =============================================================================

use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};
use std::sync::Arc;
use nalgebra_glm::{Vec3, Mat4};
use std::fmt;

// =============================================================================
// MÓDULO: COLOR
// =============================================================================

/// Estructura para representar colores en formato RGB (0-255)
#[derive(Debug, Clone, Copy)]
pub struct ColorRGB {
    pub rojo: u8,
    pub verde: u8,
    pub azul: u8,
}

impl ColorRGB {
    pub fn nuevo(r: u8, g: u8, b: u8) -> Self {
        ColorRGB { rojo: r, verde: g, azul: b }
    }

    pub fn desde_flotante(r: f32, g: f32, b: f32) -> Self {
        ColorRGB {
            rojo: (r.clamp(0.0, 1.0) * 255.0) as u8,
            verde: (g.clamp(0.0, 1.0) * 255.0) as u8,
            azul: (b.clamp(0.0, 1.0) * 255.0) as u8,
        }
    }

    pub fn a_hexadecimal(&self) -> u32 {
        ((self.rojo as u32) << 16) | ((self.verde as u32) << 8) | (self.azul as u32)
    }

    pub fn interpolar(&self, otro_color: &ColorRGB, factor: f32) -> ColorRGB {
        let t = factor.clamp(0.0, 1.0);
        ColorRGB::nuevo(
            (self.rojo as f32 * (1.0 - t) + otro_color.rojo as f32 * t) as u8,
            (self.verde as f32 * (1.0 - t) + otro_color.verde as f32 * t) as u8,
            (self.azul as f32 * (1.0 - t) + otro_color.azul as f32 * t) as u8,
        )
    }

    pub fn multiplicar(&self, escalar: f32) -> ColorRGB {
        ColorRGB::desde_flotante(
            self.rojo as f32 / 255.0 * escalar,
            self.verde as f32 / 255.0 * escalar,
            self.azul as f32 / 255.0 * escalar,
        )
    }

    pub fn sumar(&self, otro_color: &ColorRGB) -> ColorRGB {
        ColorRGB::desde_flotante(
            (self.rojo as f32 + otro_color.rojo as f32) / 255.0,
            (self.verde as f32 + otro_color.verde as f32) / 255.0,
            (self.azul as f32 + otro_color.azul as f32) / 255.0,
        )
    }
}

impl fmt::Display for ColorRGB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ColorRGB(R: {}, G: {}, B: {})", self.rojo, self.verde, self.azul)
    }
}

// =============================================================================
// MÓDULO: VERTEX
// =============================================================================

#[derive(Debug, Clone)]
pub struct Vertice {
    pub posicion: Vec3,
    pub vector_normal: Vec3,
    pub coordenadas_textura: Vec3,
    pub posicion_transformada: Vec3,
    pub normal_transformada: Vec3,
}

impl Vertice {
    pub fn nuevo(pos: Vec3, norm: Vec3, tex: Vec3) -> Self {
        Vertice {
            posicion: pos,
            vector_normal: norm,
            coordenadas_textura: tex,
            posicion_transformada: pos,
            normal_transformada: norm,
        }
    }
}

// =============================================================================
// MÓDULO: CAMERA
// =============================================================================

pub struct CamaraVirtual {
    pub ojo: Vec3,
    pub objetivo: Vec3,
    pub vector_arriba: Vec3,
}

impl CamaraVirtual {
    pub fn nueva(posicion_ojo: Vec3, punto_objetivo: Vec3, dir_arriba: Vec3) -> Self {
        CamaraVirtual { 
            ojo: posicion_ojo, 
            objetivo: punto_objetivo, 
            vector_arriba: dir_arriba 
        }
    }
}

// =============================================================================
// MÓDULO: FRAGMENT
// =============================================================================

pub struct Fragmento {
    pub posicion: Vec3,
    pub normal: Vec3,
    pub profundidad: f32,
    pub posicion_vertice: Vec3,
    pub intensidad: f32,
}

impl Fragmento {
    pub fn nuevo(
        pos: Vec3, 
        norm: Vec3, 
        prof: f32, 
        pos_vert: Vec3, 
        intens: f32
    ) -> Self {
        Fragmento {
            posicion: pos,
            normal: norm,
            profundidad: prof,
            posicion_vertice: pos_vert,
            intensidad: intens,
        }
    }
}

// =============================================================================
// MÓDULO: FRAMEBUFFER
// =============================================================================

pub struct BufferDePantalla {
    pub ancho: usize,
    pub alto: usize,
    pub buffer_colores: Vec<u32>,
    pub buffer_profundidad: Vec<f32>,
    color_fondo: u32,
    color_actual: u32,
}

impl BufferDePantalla {
    pub fn nuevo(w: usize, h: usize) -> Self {
        BufferDePantalla {
            ancho: w,
            alto: h,
            buffer_colores: vec![0; w * h],
            buffer_profundidad: vec![f32::INFINITY; w * h],
            color_fondo: 0x000000,
            color_actual: 0xFFFFFF,
        }
    }

    pub fn limpiar(&mut self) {
        for pixel in self.buffer_colores.iter_mut() {
            *pixel = self.color_fondo;
        }
        for profundidad in self.buffer_profundidad.iter_mut() {
            *profundidad = f32::INFINITY;
        }
    }

    pub fn dibujar_punto(&mut self, x: usize, y: usize, prof: f32) {
        if x < self.ancho && y < self.alto {
            let indice = y * self.ancho + x;
            if prof < self.buffer_profundidad[indice] {
                self.buffer_colores[indice] = self.color_actual;
                self.buffer_profundidad[indice] = prof;
            }
        }
    }

    pub fn establecer_color_fondo(&mut self, color: u32) {
        self.color_fondo = color;
    }

    pub fn establecer_color_actual(&mut self, color: u32) {
        self.color_actual = color;
    }
}

// =============================================================================
// MÓDULO: OBJ LOADER
// =============================================================================

pub struct ModeloOBJ {
    vertices: Vec<Vec3>,
    normales: Vec<Vec3>,
    coordenadas_uv: Vec<Vec3>,
    caras: Vec<[usize; 9]>,
}

impl ModeloOBJ {
    pub fn cargar(ruta_archivo: &str) -> Result<Self, std::io::Error> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        
        let archivo = File::open(ruta_archivo)?;
        let lector = BufReader::new(archivo);

        let mut lista_vertices = Vec::new();
        let mut lista_normales = Vec::new();
        let mut lista_uvs = Vec::new();
        let mut lista_caras = Vec::new();

        for linea in lector.lines() {
            let linea = linea?;
            let partes: Vec<&str> = linea.split_whitespace().collect();

            if partes.is_empty() {
                continue;
            }

            match partes[0] {
                "v" => {
                    if partes.len() >= 4 {
                        let x: f32 = partes[1].parse().unwrap_or(0.0);
                        let y: f32 = partes[2].parse().unwrap_or(0.0);
                        let z: f32 = partes[3].parse().unwrap_or(0.0);
                        lista_vertices.push(Vec3::new(x, y, z));
                    }
                }
                "vn" => {
                    if partes.len() >= 4 {
                        let x: f32 = partes[1].parse().unwrap_or(0.0);
                        let y: f32 = partes[2].parse().unwrap_or(0.0);
                        let z: f32 = partes[3].parse().unwrap_or(0.0);
                        lista_normales.push(Vec3::new(x, y, z));
                    }
                }
                "vt" => {
                    if partes.len() >= 3 {
                        let u: f32 = partes[1].parse().unwrap_or(0.0);
                        let v: f32 = partes[2].parse().unwrap_or(0.0);
                        lista_uvs.push(Vec3::new(u, v, 0.0));
                    }
                }
                "f" => {
                    if partes.len() >= 4 {
                        let mut cara = [0; 9];
                        for (i, parte) in partes.iter().skip(1).take(3).enumerate() {
                            let indices: Vec<&str> = parte.split('/').collect();
                            if !indices.is_empty() {
                                cara[i * 3] = indices[0].parse::<usize>().unwrap_or(1) - 1;
                            }
                            if indices.len() > 1 && !indices[1].is_empty() {
                                cara[i * 3 + 1] = indices[1].parse::<usize>().unwrap_or(1) - 1;
                            }
                            if indices.len() > 2 {
                                cara[i * 3 + 2] = indices[2].parse::<usize>().unwrap_or(1) - 1;
                            }
                        }
                        lista_caras.push(cara);
                    }
                }
                _ => {}
            }
        }

        Ok(ModeloOBJ {
            vertices: lista_vertices,
            normales: lista_normales,
            coordenadas_uv: lista_uvs,
            caras: lista_caras,
        })
    }

    pub fn obtener_array_vertices(&self) -> Vec<Vertice> {
        let mut array_vertices = Vec::new();

        for cara in &self.caras {
            for i in 0..3 {
                let idx_posicion = cara[i * 3];
                let idx_textura = cara[i * 3 + 1];
                let idx_normal = cara[i * 3 + 2];

                let posicion = self.vertices.get(idx_posicion).copied()
                    .unwrap_or(Vec3::zeros());
                let coord_tex = self.coordenadas_uv.get(idx_textura).copied()
                    .unwrap_or(Vec3::zeros());
                let normal = self.normales.get(idx_normal).copied()
                    .unwrap_or(Vec3::new(0.0, 1.0, 0.0));

                array_vertices.push(Vertice::nuevo(posicion, normal, coord_tex));
            }
        }

        array_vertices
    }
}

// =============================================================================
// MÓDULO: SHADERS (Simplificado - sin implementación completa de CPU)
// =============================================================================

pub struct UniformesCPU {
    pub projection_matrix: Mat4,
    pub view_matrix: Mat4,
    pub model_matrix: Mat4,
    pub viewport_matrix: Mat4,
    pub time: u32,
}

// =============================================================================
// APLICACIÓN PRINCIPAL CON WGPU
// =============================================================================

/// Estructura de uniformes compartida con el GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DatosUniformes {
    tiempo_actual: f32,
    tipo_render: u32,
    dimension_pantalla: [f32; 2],
    pos_planeta: [f32; 2],
    factor_escala: f32,
    _espaciado: f32,
}

/// Estructura de vértice con posición y normal
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VerticeEsfera {
    posicion: [f32; 3],
    normal: [f32; 3],
}

impl VerticeEsfera {
    fn descriptor_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<VerticeEsfera>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

fn generar_esfera(subdivisiones: u32) -> (Vec<VerticeEsfera>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for latitud in 0..=subdivisiones {
        let theta = latitud as f32 * std::f32::consts::PI / subdivisiones as f32;
        let seno_theta = theta.sin();
        let coseno_theta = theta.cos();

        for longitud in 0..=subdivisiones {
            let phi = longitud as f32 * 2.0 * std::f32::consts::PI / subdivisiones as f32;
            let seno_phi = phi.sin();
            let coseno_phi = phi.cos();

            let coord_x = seno_theta * coseno_phi;
            let coord_y = coseno_theta;
            let coord_z = seno_theta * seno_phi;

            vertices.push(VerticeEsfera {
                posicion: [coord_x, coord_y, coord_z],
                normal: [coord_x, coord_y, coord_z],
            });
        }
    }

    for lat in 0..subdivisiones {
        for lon in 0..subdivisiones {
            let primero = (lat * (subdivisiones + 1) + lon) as u16;
            let segundo = primero + subdivisiones as u16 + 1;

            indices.push(primero);
            indices.push(segundo);
            indices.push(primero + 1);

            indices.push(segundo);
            indices.push(segundo + 1);
            indices.push(primero + 1);
        }
    }

    (vertices, indices)
}

struct EstadoAplicacion {
    superficie: wgpu::Surface<'static>,
    dispositivo: wgpu::Device,
    cola_comandos: wgpu::Queue,
    configuracion: wgpu::SurfaceConfiguration,
    tamano_ventana: winit::dpi::PhysicalSize<u32>,
    pipeline_render: wgpu::RenderPipeline,
    buffer_vertices: wgpu::Buffer,
    buffer_indices: wgpu::Buffer,
    cantidad_indices: u32,
    buffer_uniformes: wgpu::Buffer,
    grupo_bind_uniformes: wgpu::BindGroup,
    datos_uniformes: DatosUniformes,
    rotacion_camara: [f32; 2],
    tiempo_inicio: std::time::Instant,
    posicion_mouse: Option<winit::dpi::PhysicalPosition<f64>>,
    mouse_presionado: bool,
}

impl EstadoAplicacion {
    async fn inicializar(ventana: Arc<Window>) -> Self {
        let tamano_ventana = ventana.inner_size();

        let instancia = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let superficie = instancia.create_surface(ventana.clone()).unwrap();

        let adaptador = instancia
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&superficie),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (dispositivo, cola_comandos) = adaptador
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let capacidades_superficie = superficie.get_capabilities(&adaptador);
        let formato_superficie = capacidades_superficie
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(capacidades_superficie.formats[0]);

        let configuracion = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: formato_superficie,
            width: tamano_ventana.width,
            height: tamano_ventana.height,
            present_mode: capacidades_superficie
                .present_modes
                .iter()
                .copied()
                .find(|m| m == &wgpu::PresentMode::Fifo)
                .unwrap_or(capacidades_superficie.present_modes[0]),
            alpha_mode: capacidades_superficie.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        superficie.configure(&dispositivo, &configuracion);

        let (vertices, indices) = generar_esfera(50);
        let cantidad_indices = indices.len() as u32;

        let buffer_vertices = dispositivo.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer de Vértices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let buffer_indices = dispositivo.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer de Índices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let datos_uniformes = DatosUniformes {
            tiempo_actual: 0.0,
            tipo_render: 1,
            dimension_pantalla: [tamano_ventana.width as f32, tamano_ventana.height as f32],
            pos_planeta: [0.0, 0.0],
            factor_escala: 0.3,
            _espaciado: 0.0,
        };

        let buffer_uniformes = dispositivo.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer de Uniformes"),
            contents: bytemuck::cast_slice(&[datos_uniformes]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let layout_bind_group_uniformes =
            dispositivo.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Layout de Bind Group de Uniformes"),
            });

        let grupo_bind_uniformes = dispositivo.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout_bind_group_uniformes,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_uniformes.as_entire_binding(),
            }],
            label: Some("Bind Group de Uniformes"),
        });

        // Shader WGSL embebido
        let codigo_shader = include_str!("shader.wgsl");
        
        let modulo_shader = dispositivo.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Módulo de Shader Principal"),
            source: wgpu::ShaderSource::Wgsl(codigo_shader.into()),
        });

        let layout_pipeline_render =
            dispositivo.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Layout del Pipeline de Render"),
                bind_group_layouts: &[&layout_bind_group_uniformes],
                push_constant_ranges: &[],
            });

        let pipeline_render = dispositivo.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipeline de Renderizado Principal"),
            layout: Some(&layout_pipeline_render),
            vertex: wgpu::VertexState {
                module: &modulo_shader,
                entry_point: "vertex_principal",
                buffers: &[VerticeEsfera::descriptor_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &modulo_shader,
                entry_point: "fragment_principal",
                targets: &[Some(wgpu::ColorTargetState {
                    format: configuracion.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            superficie,
            dispositivo,
            cola_comandos,
            configuracion,
            tamano_ventana,
            pipeline_render,
            buffer_vertices,
            buffer_indices,
            cantidad_indices,
            buffer_uniformes,
            grupo_bind_uniformes,
            datos_uniformes,
            rotacion_camara: [0.0, 0.0],
            tiempo_inicio: std::time::Instant::now(),
            posicion_mouse: None,
            mouse_presionado: false,
        }
    }

    pub fn redimensionar(&mut self, nuevo_tamano: winit::dpi::PhysicalSize<u32>) {
        if nuevo_tamano.width > 0 && nuevo_tamano.height > 0 {
            self.tamano_ventana = nuevo_tamano;
            self.configuracion.width = nuevo_tamano.width;
            self.configuracion.height = nuevo_tamano.height;
            self.superficie.configure(&self.dispositivo, &self.configuracion);
            self.datos_uniformes.dimension_pantalla = [
                nuevo_tamano.width as f32, 
                nuevo_tamano.height as f32
            ];
        }
    }

    fn procesar_mouse_click(&mut self, presionado: bool) {
        self.mouse_presionado = presionado;
    }

    fn procesar_movimiento_mouse(&mut self, posicion: winit::dpi::PhysicalPosition<f64>) {
        if self.mouse_presionado {
            if let Some(pos_anterior) = self.posicion_mouse {
                let delta_x = (posicion.x - pos_anterior.x) as f32;
                let delta_y = (posicion.y - pos_anterior.y) as f32;
                
                // Sensibilidad del mouse
                self.rotacion_camara[0] += delta_x * 0.005;
                self.rotacion_camara[1] = (self.rotacion_camara[1] - delta_y * 0.005)
                    .clamp(-1.5, 1.5);
            }
        }
        self.posicion_mouse = Some(posicion);
    }

    fn actualizar(&mut self) {
        self.datos_uniformes.tiempo_actual = self.tiempo_inicio.elapsed().as_secs_f32();
        self.cola_comandos.write_buffer(
            &self.buffer_uniformes,
            0,
            bytemuck::cast_slice(&[self.datos_uniformes]),
        );
    }

    fn renderizar(&mut self) -> Result<(), wgpu::SurfaceError> {
        let salida = self.superficie.get_current_texture()?;
        let vista = salida
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut codificador = self
            .dispositivo
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Codificador de Comandos de Render"),
            });

        // Configuración: [posición_x, posición_y, escala, tipo_shader]
        // Tipos: 1=Sol, 2=Rocoso(Marte), 3=Gaseoso(Júpiter), 4=Anillos(Saturno), 5=Volcánico, 6=Luna(Hielo)
        let configuracion_planetas = [
            [0.0, 0.0, 0.55, 1.0],      // Centro: Sol (amarillo-naranja brillante)
            [-0.6, 0.35, 0.12, 2.0],    // Izq arriba: Marte (pequeño, rojo)
            [0.65, -0.25, 0.38, 4.0],   // Der abajo: Saturno (grande con anillos)
            [-0.3, -0.5, 0.18, 6.0],    // Izq abajo: Luna helada (azul-blanco)
        ];

        let datos_planetas: Vec<_> = configuracion_planetas
            .iter()
            .map(|config_planeta| {
                let mut uniformes_planeta = self.datos_uniformes;
                uniformes_planeta.pos_planeta = [config_planeta[0], config_planeta[1]];
                uniformes_planeta.factor_escala = config_planeta[2];
                uniformes_planeta.tipo_render = config_planeta[3] as u32;

                let buffer_uniforme_planeta = self.dispositivo.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("Buffer de Uniformes de Planeta"),
                        contents: bytemuck::cast_slice(&[uniformes_planeta]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    }
                );

                let bind_group_planeta = self.dispositivo.create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        layout: &self.pipeline_render.get_bind_group_layout(0),
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buffer_uniforme_planeta.as_entire_binding(),
                        }],
                        label: Some("Bind Group de Planeta"),
                    }
                );

                (buffer_uniforme_planeta, bind_group_planeta)
            })
            .collect();

        {
            let mut pase_render = codificador.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Pase de Renderizado Principal"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &vista,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.08,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pase_render.set_pipeline(&self.pipeline_render);
            pase_render.set_vertex_buffer(0, self.buffer_vertices.slice(..));
            pase_render.set_index_buffer(self.buffer_indices.slice(..), wgpu::IndexFormat::Uint16);

            for i in 0..200 {
                let posicion_x = (i as f32 * 567.123).sin() * 2.0;
                let posicion_y = (i as f32 * 432.567).cos() * 2.0;
                let tamano_estrella = ((i as f32 * 789.345).sin() * 0.5 + 0.5) * 0.003;
                
                let mut uniformes_estrella = self.datos_uniformes;
                uniformes_estrella.pos_planeta = [posicion_x, posicion_y];
                uniformes_estrella.factor_escala = tamano_estrella;
                uniformes_estrella.tipo_render = 7;

                self.cola_comandos.write_buffer(
                    &self.buffer_uniformes, 
                    0, 
                    bytemuck::cast_slice(&[uniformes_estrella])
                );
                pase_render.set_bind_group(0, &self.grupo_bind_uniformes, &[]);
                pase_render.draw_indexed(0..self.cantidad_indices, 0, 0..1);
            }

            for (indice, (buffer_planeta, bind_group_planeta)) in datos_planetas.iter().enumerate() {
                let planeta = configuracion_planetas[indice];

                let mut uniformes_planeta = self.datos_uniformes;
                uniformes_planeta.pos_planeta = [
                    planeta[0] * self.rotacion_camara[0].cos() 
                        - planeta[2] * self.rotacion_camara[0].sin(),
                    planeta[1] * self.rotacion_camara[1].cos()
                ];
                uniformes_planeta.factor_escala = planeta[2] * 
                    (0.8 + 0.2 * (self.rotacion_camara[0].cos() 
                               * self.rotacion_camara[1].cos()));
                uniformes_planeta.tipo_render = planeta[3] as u32;

                self.cola_comandos.write_buffer(
                    buffer_planeta, 
                    0, 
                    bytemuck::cast_slice(&[uniformes_planeta])
                );
                pase_render.set_bind_group(0, bind_group_planeta, &[]);
                pase_render.draw_indexed(0..self.cantidad_indices, 0, 0..1);
            }
        }

        self.cola_comandos.submit(std::iter::once(codificador.finish()));
        salida.present();

        Ok(())
    }
}

fn main() {
    env_logger::init();
    
    let loop_eventos = EventLoop::new().unwrap();
    let ventana = Arc::new(
        winit::window::WindowBuilder::new()
            .with_title("Sistema Solar - Pablo Cabrera 231156")
            .with_inner_size(winit::dpi::LogicalSize::new(1000, 800))
            .build(&loop_eventos)
            .unwrap(),
    );

    let mut estado = pollster::block_on(EstadoAplicacion::inicializar(ventana.clone()));

    println!("===========================================");
    println!("Sistema Solar Interactivo - TODO EN UNO");
    println!("Autor: Pablo Cabrera - Carné: 231156");
    println!("===========================================");
    println!("Controles:");
    println!("  Click y arrastra: Rotar cámara");
    println!("  ESC: Salir");
    println!("===========================================");

    loop_eventos
        .run(move |evento, control_flujo| {
            match evento {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == ventana.id() => match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => control_flujo.exit(),
                    WindowEvent::Resized(tamano_fisico) => {
                        estado.redimensionar(*tamano_fisico);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        estado.procesar_movimiento_mouse(*position);
                    }
                    WindowEvent::MouseInput { state: mouse_state, button: winit::event::MouseButton::Left, .. } => {
                        estado.procesar_mouse_click(*mouse_state == ElementState::Pressed);
                    }
                    WindowEvent::RedrawRequested => {
                        estado.actualizar();
                        match estado.renderizar() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => estado.redimensionar(estado.tamano_ventana),
                            Err(wgpu::SurfaceError::OutOfMemory) => control_flujo.exit(),
                            Err(e) => eprintln!("Error de renderizado: {:?}", e),
                        }
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    ventana.request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}