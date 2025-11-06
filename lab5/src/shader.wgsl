// =============================================================================
// Shaders GPU para Planetas Procedurales (WGSL)
// Autor: Pablo Cabrera
// Carné: 231156
// Descripción: Shaders de vértices y fragmentos para renderizado en GPU
// =============================================================================

// Estructura de uniformes compartida entre shaders
struct UniformesPlaneta {
    tiempo: f32,
    tipo_shader: u32,
    resolucion: vec2<f32>,
    posicion_planeta: vec2<f32>,
    escala_planeta: f32,
    _relleno: f32,
}

@group(0) @binding(0)
var<uniform> uniformes: UniformesPlaneta;

// Estructura de entrada del vertex shader
struct EntradaVertice {
    @location(0) posicion: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

// Estructura de salida del vertex shader
struct SalidaVertice {
    @builtin(position) posicion_clip: vec4<f32>,
    @location(0) pos_mundo: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

// =============================================================================
// VERTEX SHADER - Transforma vértices a espacio de pantalla
// =============================================================================

@vertex
fn vertex_principal(entrada: EntradaVertice) -> SalidaVertice {
    var salida: SalidaVertice;
    
    // Aplicar rotación automática con el tiempo
    let angulo = uniformes.tiempo * 0.3;
    let cos_angulo = cos(angulo);
    let sin_angulo = sin(angulo);
    
    // Matriz de rotación en eje Y
    let rotacion_y = mat3x3<f32>(
        vec3<f32>(cos_angulo, 0.0, sin_angulo),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(-sin_angulo, 0.0, cos_angulo)
    );
    
    // Escalar y rotar la posición
    let pos_escalada = entrada.posicion * uniformes.escala_planeta;
    let pos_rotada = rotacion_y * pos_escalada;
    let normal_rotada = rotacion_y * entrada.normal;
    
    // Proyección simple con offset de posición del planeta
    let posicion_final = pos_rotada * vec3<f32>(1.0, 1.0, 0.5);
    salida.posicion_clip = vec4<f32>(
        posicion_final.xy + uniformes.posicion_planeta, 
        0.5, 
        1.0
    );
    salida.pos_mundo = pos_rotada;
    salida.normal = normalize(normal_rotada);
    
    return salida;
}

// =============================================================================
// FUNCIONES AUXILIARES DE RUIDO Y PATRONES
// =============================================================================

/// Función hash para generar valores pseudo-aleatorios
fn hash_3d(p: vec3<f32>) -> f32 {
    let h = sin(p.x * 127.1 + p.y * 311.7 + p.z * 74.7) * 43758.5453;
    return fract(h);
}

/// Ruido de valor suavizado
fn ruido_suave(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    
    // Interpolación hermite suave
    let u = f * f * (3.0 - 2.0 * f);
    
    // Valores en las esquinas del cubo
    let a = hash_3d(i + vec3<f32>(0.0, 0.0, 0.0));
    let b = hash_3d(i + vec3<f32>(1.0, 0.0, 0.0));
    let c = hash_3d(i + vec3<f32>(0.0, 1.0, 0.0));
    let d = hash_3d(i + vec3<f32>(1.0, 1.0, 0.0));
    
    // Interpolación bilineal
    let x1 = mix(a, b, u.x);
    let x2 = mix(c, d, u.x);
    
    return mix(x1, x2, u.y);
}

/// Fractal Brownian Motion - Ruido en múltiples octavas
fn fbm_ruido(p: vec3<f32>, octavas: i32) -> f32 {
    var valor = 0.0;
    var amplitud = 0.5;
    var frecuencia = 1.0;
    var posicion = p;
    
    for (var i = 0; i < octavas; i++) {
        valor += amplitud * ruido_suave(posicion * frecuencia);
        frecuencia *= 2.0;
        amplitud *= 0.5;
    }
    
    return valor;
}

/// Patrón de Voronoi simplificado para texturizar
fn patron_voronoi(p: vec3<f32>) -> f32 {
    let celda_int = floor(p);
    let celda_frac = fract(p);
    
    var dist_minima = 2.0;
    
    // Buscar punto más cercano en celdas vecinas
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let vecino = vec3<f32>(f32(i), f32(j), 0.0);
            let punto = vecino + vec3<f32>(
                hash_3d(celda_int + vecino),
                hash_3d(celda_int + vecino + vec3<f32>(0.1, 0.1, 0.1)),
                0.0
            );
            let diferencia = punto - celda_frac;
            let distancia = dot(diferencia.xy, diferencia.xy);
            dist_minima = min(dist_minima, distancia);
        }
    }
    
    return sqrt(dist_minima);
}

// =============================================================================
// SHADERS DE PLANETAS ESPECÍFICOS
// =============================================================================

/// Shader 1: Sol con plasma animado
fn shader_sol(pos: vec3<f32>, t: f32) -> vec3<f32> {
    let p = pos * 3.0;
    
    // Sistema de plasma multicapa con más variación
    let plasma1 = fbm_ruido(p + vec3<f32>(t, t * 0.5, 0.0), 5);
    let plasma2 = fbm_ruido(p * 2.0 - vec3<f32>(t * 0.7, t * 1.1, t * 0.3), 4);
    let plasma3 = fbm_ruido(p * 0.5 + vec3<f32>(cos(t * 0.5), sin(t * 0.5), t * 0.2), 4);
    let plasma4 = fbm_ruido(p * 4.0 + vec3<f32>(t * 0.3, t * 0.8, 0.0), 3);
    
    let combinado = clamp((plasma1 * 0.4 + plasma2 * 0.3 + plasma3 * 0.2 + plasma4 * 0.1), 0.0, 1.0);
    
    // Vórtices espirales
    let angulo = atan2(p.y, p.x);
    let radio = length(p.xy);
    let espiral = sin(angulo * 6.0 + radio * 4.0 - t * 2.0 + combinado * 2.0) * 0.5 + 0.5;
    
    // Manchas solares (zonas oscuras)
    let ruido_manchas = fbm_ruido(p * 3.0 + vec3<f32>(t * 0.05, 0.0, 0.0), 3);
    let manchas = select(1.0, 0.4, ruido_manchas > 0.68);
    
    // Corona pulsante
    let distancia = length(p);
    let corona = pow(1.0 - distancia * 0.4, 3.0) * 1.5;
    let pulso = sin(t * 2.0) * 0.2 + 1.2;
    
    // Gradiente de temperatura (paleta naranja-amarillo brillante)
    let temperatura = combinado * espiral * 1.2;
    var color_base: vec3<f32>;
    
    if (temperatura > 0.8) {
        color_base = vec3<f32>(1.0, 0.95, 0.7); // Blanco amarillento brillante
    } else if (temperatura > 0.6) {
        color_base = vec3<f32>(1.0, 0.85, 0.3); // Amarillo intenso
    } else if (temperatura > 0.4) {
        color_base = vec3<f32>(1.0, 0.6, 0.1);  // Naranja brillante
    } else {
        color_base = vec3<f32>(0.95, 0.4, 0.05); // Naranja-rojo
    }
    
    return color_base * manchas * (1.0 + corona * pulso * 0.8);
}

/// Shader 2: Planeta rocoso tipo Marte
fn shader_rocoso(pos: vec3<f32>, t: f32) -> vec3<f32> {
    let p = pos * 5.0;
    
    // Generación de terreno más compleja con Voronoi
    let continentes = patron_voronoi(p * 0.8);
    let montanas = fbm_ruido(p * 3.0, 3) * 0.3;
    let detalles_finos = fbm_ruido(p * 8.0, 2) * 0.15;
    let altura_terreno = continentes * 0.55 + montanas * 0.3 + detalles_finos * 0.15;
    
    var color_terreno: vec3<f32>;
    
    // Paleta de colores tipo Marte más variada
    if (altura_terreno < 0.35) {
        // Planicies bajas con variación
        let var_planicies = fbm_ruido(p * 6.0, 2);
        if (var_planicies > 0.6) {
            color_terreno = vec3<f32>(0.85, 0.35, 0.15); // Rojo óxido claro
        } else {
            color_terreno = vec3<f32>(0.7, 0.25, 0.1);   // Rojo óxido oscuro
        }
    } else if (altura_terreno >= 0.65) {
        // Casquetes polares de hielo
        color_terreno = vec3<f32>(0.95, 0.95, 1.0);
    } else if (altura_terreno >= 0.55) {
        // Montañas con más detalle
        let roca_var = fbm_ruido(p * 10.0, 2);
        if (roca_var > 0.5) {
            color_terreno = vec3<f32>(0.65, 0.22, 0.12); // Roca marrón-rojo
        } else {
            color_terreno = vec3<f32>(0.5, 0.18, 0.09);  // Roca oscura
        }
    } else {
        // Terreno variado con más texturas
        let veg = fbm_ruido(p * 5.0, 3);
        if (veg > 0.65) {
            color_terreno = vec3<f32>(0.75, 0.32, 0.12); // Rojo claro arenoso
        } else if (veg > 0.4) {
            color_terreno = vec3<f32>(0.6, 0.22, 0.11);  // Rojo medio
        } else {
            color_terreno = vec3<f32>(0.45, 0.16, 0.09); // Rojo muy oscuro
        }
    }

    // Tormentas de polvo animadas
    let tormenta1 = fbm_ruido(p * 2.0 + vec3<f32>(t * 15.0, 0.0, t * 8.0), 3);
    let tormenta2 = fbm_ruido(p * 4.0 - vec3<f32>(t * 10.0, 0.0, t * 5.0), 2);
    let polvo = clamp((tormenta1 * 0.7 + tormenta2 * 0.3), 0.0, 1.0);

    // Aplicar tormentas de polvo
    if (polvo > 0.6) {
        let densidad = min((polvo - 0.6) / 0.4, 1.0);
        color_terreno = mix(color_terreno, vec3<f32>(0.8, 0.4, 0.2), densidad * 0.5);
    }
    
    return color_terreno;
}

/// Shader 3: Gigante gaseoso tipo Júpiter
fn shader_gaseoso(pos: vec3<f32>, t: f32) -> vec3<f32> {
    let p = pos * 3.5;
    
    // Bandas atmosféricas horizontales
    let bandas_base = p.y * 18.0;
    let turb1 = fbm_ruido(p * 2.0 + vec3<f32>(t * 1.5, 0.0, 0.0), 3) * 2.0;
    let turb2 = fbm_ruido(p * 4.0 - vec3<f32>(t * 0.8, 0.0, t * 0.5), 2) * 0.8;
    
    let pos_banda = bandas_base + turb1 + turb2;
    let bandas = sin(pos_banda) * 0.5 + 0.5;
    
    let caos_atmosferico = fbm_ruido(p * 3.0 + vec3<f32>(t, 0.0, 0.0), 3);
    let valor_banda = clamp(bandas * 0.6 + caos_atmosferico * 0.4, 0.0, 1.0);
    
    // Paleta de colores joviana vibrante
    let tono1 = vec3<f32>(0.95, 0.85, 0.65);  // Crema claro
    let tono2 = vec3<f32>(0.85, 0.5, 0.3);    // Naranja terracota
    let tono3 = vec3<f32>(0.9, 0.7, 0.5);     // Beige dorado
    let tono4 = vec3<f32>(1.0, 0.9, 0.7);     // Blanco cremoso
    
    var color_final: vec3<f32>;
    
    if (valor_banda < 0.25) {
        color_final = mix(tono1, tono2, valor_banda * 4.0);
    } else if (valor_banda < 0.5) {
        color_final = mix(tono2, tono3, (valor_banda - 0.25) * 4.0);
    } else if (valor_banda < 0.75) {
        color_final = mix(tono3, tono4, (valor_banda - 0.5) * 4.0);
    } else {
        color_final = mix(tono4, tono1, (valor_banda - 0.75) * 4.0);
    }
    
    // Gran Mancha Roja (en lugar de blanca)
    let centro_mancha = vec3<f32>(0.6, -0.3, 0.0);
    let dx = p.x - centro_mancha.x;
    let dy = (p.y - centro_mancha.y) * 1.4;
    let dz = p.z - centro_mancha.z;
    let dist_mancha = sqrt(dx * dx + dy * dy + dz * dz);
    
    if (dist_mancha < 0.5) {
        let factor_mancha = max(1.0 - dist_mancha / 0.5, 0.0);
        let angulo = atan2(dy, dx);
        let remolino = sin(angulo * 5.0 + dist_mancha * 15.0 - t * 3.0) * 0.5 + 0.5;
        
        let intensidad_rojo = factor_mancha * (0.7 + remolino * 0.3);
        let color_rojo = select(
            vec3<f32>(0.9, 0.2, 0.1),   // Rojo oscuro intenso
            vec3<f32>(1.0, 0.3, 0.15),  // Rojo brillante
            remolino > 0.6
        );
        
        color_final = mix(color_final, color_rojo, intensidad_rojo * 0.95);
    }
    
    return color_final;
}

/// Shader 4: Planeta con anillos tipo Saturno
fn shader_anillos(pos: vec3<f32>, t: f32) -> vec3<f32> {
    let p = pos * 3.0;
    
    // Planeta base con bandas
    let bandas = sin(p.y * 20.0 + fbm_ruido(p, 2) * 0.5) * 0.5 + 0.5;
    let tono1 = vec3<f32>(0.5, 0.3, 0.7);  // Púrpura oscuro
    let tono2 = vec3<f32>(0.7, 0.5, 0.9);  // Púrpura claro
    var color_planeta = mix(tono1, tono2, bandas);

    // Sistema de anillos espectaculares
    let dist_anillo = length(p.xz);
    let altura_y = abs(p.y);
    
    if (altura_y < 0.18 && dist_anillo > 0.75 && dist_anillo < 2.0) {
        let freq_anillos = dist_anillo * 50.0;
        let bandas_anillo = sin(freq_anillos) * 0.5 + 0.5;
        let var_brillo = sin(dist_anillo * 30.0 + t * 3.0) * 0.5 + 0.5;
        
        // Textura adicional en los anillos
        let textura_anillos = fbm_ruido(vec3<f32>(dist_anillo * 20.0, p.y * 50.0, t * 0.5), 3);
        let bandas_ajustadas = bandas_anillo * 0.7 + textura_anillos * 0.3;
        
        // Gaps de Cassini (divisiones oscuras)
        let es_gap = (dist_anillo > 1.0 && dist_anillo < 1.15) ||
                     (dist_anillo > 1.5 && dist_anillo < 1.55) ||
                     (dist_anillo > 1.75 && dist_anillo < 1.78);
        
        if (es_gap) {
            color_planeta *= 0.4;
        } else {
            var color_anillo: vec3<f32>;
            if (bandas_ajustadas > 0.7) {
                color_anillo = vec3<f32>(1.0, 0.9, 0.6);  // Dorado claro
            } else if (bandas_ajustadas > 0.4) {
                color_anillo = vec3<f32>(0.95, 0.7, 0.8); // Rosa dorado
            } else {
                color_anillo = vec3<f32>(0.8, 0.5, 0.6);  // Rosa oscuro
            }
            
            let alpha_anillo = (1.0 - pow(altura_y / 0.18, 1.2)) * 0.95;
            let brillo_anillo = 0.9 + var_brillo * 0.2;
            
            color_planeta = mix(color_planeta, color_anillo, alpha_anillo);
            color_planeta *= brillo_anillo;
        }
    }
    
    // Sombra de anillos sobre el planeta
    if (altura_y < 0.2 && dist_anillo < 0.9) {
        let bandas_sombra = sin(dist_anillo * 50.0) * 0.5 + 0.5;
        let sombra = 0.6 + bandas_sombra * 0.3;
        color_planeta *= sombra;
    }
    
    return color_planeta;
}

/// Shader 5: Planeta volcánico con lava
fn shader_volcanico(pos: vec3<f32>, t: f32) -> vec3<f32> {
    let p = pos * 4.0;
    
    // Superficie agrietada
    let grietas = patron_voronoi(p * 1.5);
    let grietas_finas = fbm_ruido(p * 8.0 + vec3<f32>(t, 0.0, 0.0), 3);
    
    let es_lava = grietas < 0.4 || grietas_finas > 0.8;
    
    var color_superficie: vec3<f32>;
    
    // Colores de lava naranja-roja intensa
    if (es_lava) {
        let calor = fbm_ruido(p * 2.0 + vec3<f32>(t * 2.0, 0.0, t), 3);
        let pulso = sin(t * 5.0) * 0.25 + 0.75;
        
        if (calor > 0.75) {
            color_superficie = vec3<f32>(1.0, 1.0, 0.8) * pulso;  // Blanco incandescente
        } else if (calor > 0.55) {
            color_superficie = vec3<f32>(1.0, 0.9, 0.3) * pulso;  // Amarillo brillante
        } else if (calor > 0.35) {
            color_superficie = vec3<f32>(1.0, 0.5, 0.1) * pulso;  // Naranja intenso
        } else {
            color_superficie = vec3<f32>(0.9, 0.2, 0.05) * pulso; // Rojo lava
        }
    } else {
        // Roca solidificada
        let variacion = fbm_ruido(p * 10.0, 2);
        if (variacion > 0.6) {
            color_superficie = vec3<f32>(0.1, 0.2, 0.2);  // Roca oscura
        } else {
            color_superficie = vec3<f32>(0.05, 0.1, 0.1); // Roca muy oscura
        }
    }
    
    return color_superficie;
}

/// Shader 6: Luna con cráteres de hielo
fn shader_luna(pos: vec3<f32>) -> vec3<f32> {
    let p = pos * 5.0;
    
    // Cráteres con Voronoi
    let patron_crater = patron_voronoi(p * 1.2);
    let es_crater = patron_crater < 0.25;
    
    // Mares lunares
    let patron_mar = fbm_ruido(p * 0.8, 3);
    let es_mar = patron_mar < 0.3;
    
    // Tierras altas
    let tierras = fbm_ruido(p * 2.0, 2);
    let es_tierras_altas = tierras > 0.7;
    
    var color_superficie: vec3<f32>;
    
    // Paleta de hielo con más variación
    if (es_crater) {
        // Cráteres con grietas
        let grietas = fbm_ruido(p * 15.0, 2);
        if (grietas > 0.6) {
            color_superficie = vec3<f32>(0.65, 0.75, 0.85); // Hielo agrietado claro
        } else {
            color_superficie = vec3<f32>(0.7, 0.8, 0.9);    // Hielo agrietado
        }
    } else if (es_mar) {
        color_superficie = vec3<f32>(0.5, 0.7, 0.8);   // Hielo azulado
    } else if (es_tierras_altas) {
        color_superficie = vec3<f32>(0.9, 0.95, 1.0);  // Hielo brillante
    } else {
        // Hielo base con variación
        let variacion_hielo = fbm_ruido(p * 6.0, 2);
        if (variacion_hielo > 0.5) {
            color_superficie = vec3<f32>(0.85, 0.92, 0.97); // Hielo claro
        } else {
            color_superficie = vec3<f32>(0.75, 0.85, 0.92); // Hielo medio
        }
    }
    
    return color_superficie;
}

/// Shader 7: Estrellas de fondo
fn shader_estrella(pos: vec3<f32>, t: f32) -> vec3<f32> {
    let dist = length(pos);
    let brillo = pow(1.0 - dist, 8.0);
    let parpadeo = sin(t * 3.0 + pos.x * 10.0 + pos.y * 8.0) * 0.5 + 0.5;
    return vec3<f32>(1.0, 1.0, 1.0) * brillo * (0.7 + parpadeo * 0.3);
}

// =============================================================================
// FRAGMENT SHADER PRINCIPAL
// =============================================================================

@fragment
fn fragment_principal(entrada: SalidaVertice) -> @location(0) vec4<f32> {
    let pos_normalizada = normalize(entrada.pos_mundo);
    let normal_normalizada = normalize(entrada.normal);
    let t = uniformes.tiempo;
    
    var color_final: vec3<f32>;
    
    // Seleccionar shader según tipo
    switch uniformes.tipo_shader {
        case 1u: { color_final = shader_sol(pos_normalizada, t); }
        case 2u: { color_final = shader_rocoso(pos_normalizada, t); }
        case 3u: { color_final = shader_gaseoso(pos_normalizada, t); }
        case 4u: { color_final = shader_anillos(pos_normalizada, t); }
        case 5u: { color_final = shader_volcanico(pos_normalizada, t); }
        case 6u: { color_final = shader_luna(pos_normalizada); }
        case 7u: { color_final = shader_estrella(pos_normalizada, t); }
        default: { color_final = vec3<f32>(1.0, 0.0, 1.0); }
    }
    
    // Iluminación básica direccional
    let direccion_luz = normalize(vec3<f32>(1.0, 0.5, 0.8));
    let difusa = max(dot(normal_normalizada, direccion_luz), 0.15);
    
    // Auto-emisión para sol y lava
    let emision = select(1.0, difusa, uniformes.tipo_shader != 1u && uniformes.tipo_shader != 5u);
    
    return vec4<f32>(color_final * mix(1.0, difusa, 0.7), 1.0);
}
