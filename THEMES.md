# 🎨 RustyVault - Themes & Colors Guide

**¿El azul te molesta? ¡Aquí está la solución!**

## 🚀 **CAMBIO RÁPIDO DE TEMA**

### 📂 **Ubicación**: `src/main.rs` - línea ~130

```rust
/// Configurar estilo custom para egui - Dark Mode elegante
fn setup_custom_style(ctx: &egui::Context) {
    // 🎨 CAMBIO DE TEMA: Cambia esta línea para probar diferentes temas
    setup_theme_elegant_dark(ctx);   // ← CAMBIA AQUÍ
    
    // 🌟 TEMAS DISPONIBLES:
    // setup_theme_elegant_dark(ctx);   // ✨ Gris violeta suave (ACTUAL)
    // setup_theme_forest_green(ctx);   // 🟢 Verde oscuro profesional
    // setup_theme_steel_blue(ctx);     // 🔵 Azul acero suave
}
```

---

## 🎨 **TEMAS DISPONIBLES**

### ✨ **Elegant Dark** (Recomendado)
**Actual:** Gris violeta suave, sin azul molesto
- **Base**: Grises oscuros elegantes
- **Accent**: Violeta sutil para elementos activos
- **Selección**: Gris violeta para texto seleccionado
- **Uso**: `setup_theme_elegant_dark(ctx);`

### 🟢 **Forest Green** 
**Profesional:** Verde oscuro relajante
- **Base**: Tonos verdes oscuros
- **Accent**: Verde suave para elementos activos
- **Selección**: Verde elegante
- **Uso**: `setup_theme_forest_green(ctx);`

### 🔵 **Steel Blue**
**Elegante:** Azul acero suave (NO brillante)
- **Base**: Azul gris oscuro
- **Accent**: Azul acero suave (sin el azul molesto)
- **Selección**: Azul elegante y suave
- **Uso**: `setup_theme_steel_blue(ctx);`

---

## 🔧 **CÓMO CAMBIAR EL TEMA**

### **Paso 1**: Abrir el archivo
```bash
# Con VS Code
code src/main.rs

# O tu editor favorito
notepad src/main.rs
```

### **Paso 2**: Buscar la función `setup_custom_style`
- Línea aproximada: ~130
- Buscar: `setup_theme_elegant_dark(ctx);`

### **Paso 3**: Cambiar la línea
```rust
// ANTES (gris violeta)
setup_theme_elegant_dark(ctx);

// DESPUÉS (verde profesional)
setup_theme_forest_green(ctx);

// O (azul acero suave)
setup_theme_steel_blue(ctx);
```

### **Paso 4**: Recompilar y ejecutar
```bash
cargo build
cargo run
```

---

## 🎯 **PERSONALIZACIÓN AVANZADA**

### **Crear tu propio tema**

Si quieres crear tu propio esquema de colores, duplica una de las funciones:

```rust
/// 🌟 MI TEMA PERSONALIZADO
fn setup_theme_mi_estilo(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    style.visuals.dark_mode = true;
    
    // TUS COLORES AQUÍ
    style.visuals.window_fill = egui::Color32::from_rgb(R, G, B);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(R, G, B);
    
    apply_common_style_settings(&mut style);
    ctx.set_style(style);
}
```

### **Colores importantes a cambiar:**

1. **`widgets.active.bg_fill`** - Color cuando seleccionas tabs
2. **`selection.bg_fill`** - Color del texto seleccionado
3. **`hyperlink_color`** - Color de los links

---

## 🚨 **PROBLEMA RESUELTO**

### **ANTES**: Azul molesto
```rust
// Este azul brillante era horrible:
style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 120, 215); // 😵 AZUL MOLESTO
```

### **DESPUÉS**: Colores elegantes
```rust
// Ahora con colores suaves:
style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 90); // ✨ GRIS VIOLETA SUAVE
```

---

## 💡 **TIPS & RECOMENDACIONES**

### **🌟 Para la mayoría de usuarios:**
- **Usa**: `setup_theme_elegant_dark(ctx);`
- **¿Por qué?**: Violeta suave, elegante, no cansa la vista

### **🟢 Si prefieres verde:**
- **Usa**: `setup_theme_forest_green(ctx);`
- **¿Por qué?**: Verde es relajante y profesional

### **🔵 Si te gusta el azul (pero NO molesto):**
- **Usa**: `setup_theme_steel_blue(ctx);`
- **¿Por qué?**: Azul acero suave, sin la intensidad molesta

### **🔄 Experimentar:**
- Prueba los 3 temas y quédate con el que más te guste
- Es súper fácil cambiar

---

## 🎨 **VALORES RGB EXACTOS**

### **Elegant Dark (Actual)**
```rust
Active: rgb(80, 80, 90)    // Gris violeta suave
Selection: rgb(90, 90, 100) // Gris violeta selección
Hyperlink: rgb(140, 140, 180) // Violeta suave links
```

### **Forest Green**
```rust
Active: rgb(70, 85, 70)    // Verde suave
Selection: rgb(80, 100, 80) // Verde selección
Hyperlink: rgb(120, 160, 120) // Verde links
```

### **Steel Blue**
```rust
Active: rgb(75, 80, 90)    // Azul acero suave
Selection: rgb(85, 90, 100) // Azul acero selección
Hyperlink: rgb(130, 140, 170) // Azul acero links
```

---

**¡Disfruta de colores que no lastimen tus ojos! 👀✨**
