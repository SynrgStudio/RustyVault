fn parse_robocopy_stats(output: &str) -> (i32, u64) {
    println!("🔍 DEBUG: Parsing robocopy output:");
    println!("---START---");
    println!("{}", output);
    println!("---END---");
    
    let mut files_copied = 0;
    let mut bytes_transferred = 0;
    
    for line in output.lines() {
        let line = line.trim();
        println!("🔍 DEBUG: Processing line: '{}'", line);
        
        // Buscar línea de archivos en español: " Archivos:         1         0         1         0         0         0"
        if line.starts_with("Archivos:") {
            println!("🔍 DEBUG: Found Archivos line: '{}'", line);
            
            // Dividir por espacios en blanco y filtrar vacíos
            let parts: Vec<&str> = line.split_whitespace().collect();
            println!("🔍 DEBUG: Line parts: {:?}", parts);
            
            if parts.len() >= 3 {
                // parts[0] = "Archivos:"
                // parts[1] = Total
                // parts[2] = Copiado (lo que necesitamos)
                if let Ok(copied) = parts[2].parse::<i32>() {
                    files_copied = copied;
                    println!("✅ DEBUG: Files copied parsed: {}", files_copied);
                } else {
                    println!("❌ DEBUG: Failed to parse files copied from: '{}'", parts[2]);
                }
            } else {
                println!("❌ DEBUG: Not enough parts in Archivos line: {} parts", parts.len());
            }
        }
        
        // Buscar línea de bytes en español: "    Bytes:    28.9 k    14.4 k    14.4 k         0         0         0"
        if line.starts_with("Bytes:") {
            println!("🔍 DEBUG: Found Bytes line: '{}'", line);
            
            // Usar regex o parsing manual para encontrar patrones como "14.4 k"
            // Primero intentemos encontrar el primer número con posible sufijo después de "Bytes:"
            let after_bytes = &line[6..]; // Skip "Bytes:"
            println!("🔍 DEBUG: After 'Bytes:': '{}'", after_bytes);
            
            // Buscar el primer patrón de número seguido opcionalmente de sufijo
            let parts: Vec<&str> = after_bytes.split_whitespace().collect();
            println!("🔍 DEBUG: Bytes parts: {:?}", parts);
            
            // Estructura: Total, Copiado, Omitido, ...
            // Queremos los bytes copiados (segunda columna)
            if parts.len() >= 4 {
                let first_part = parts[0];  // Total (28.9)
                let first_suffix = parts[1]; // k
                let second_part = parts[2]; // Copiado (14.4)
                let second_suffix = parts[3]; // k
                
                // Verificar si el segundo part es un sufijo válido
                if ["k", "m", "g", "t"].contains(&second_suffix.to_lowercase().as_str()) {
                    let combined = format!("{}{}", second_part, second_suffix);
                    println!("🔍 DEBUG: Trying combined COPIED size: '{}'", combined);
                    if let Ok(size) = parse_robocopy_size(&combined) {
                        bytes_transferred = size;
                        println!("✅ DEBUG: Bytes transferred (COPIED) parsed: {}", bytes_transferred);
                    } else {
                        println!("❌ DEBUG: Failed to parse combined copied bytes: '{}'", combined);
                    }
                } else {
                    // Fallback: intentar parsear solo el segundo part (copiado)
                    if let Ok(size) = parse_robocopy_size(second_part) {
                        bytes_transferred = size;
                        println!("✅ DEBUG: Bytes transferred (COPIED) parsed: {}", bytes_transferred);
                    } else {
                        println!("❌ DEBUG: Failed to parse copied bytes from: '{}'", second_part);
                    }
                }
            } else if parts.len() >= 2 {
                // Fallback para formato simple
                let first_part = parts[0];
                let second_part = parts[1];
                
                if ["k", "m", "g", "t"].contains(&second_part.to_lowercase().as_str()) {
                    let combined = format!("{}{}", first_part, second_part);
                    println!("🔍 DEBUG: Trying combined size: '{}'", combined);
                    if let Ok(size) = parse_robocopy_size(&combined) {
                        bytes_transferred = size;
                        println!("✅ DEBUG: Bytes transferred parsed: {}", bytes_transferred);
                    } else {
                        println!("❌ DEBUG: Failed to parse combined bytes: '{}'", combined);
                    }
                } else {
                    // Intentar parsear solo el primer part
                    if let Ok(size) = parse_robocopy_size(first_part) {
                        bytes_transferred = size;
                        println!("✅ DEBUG: Bytes transferred parsed: {}", bytes_transferred);
                    } else {
                        println!("❌ DEBUG: Failed to parse bytes from: '{}'", first_part);
                    }
                }
            } else {
                println!("❌ DEBUG: Not enough parts in Bytes line: {} parts", parts.len());
            }
        }
    }
    
    println!("🎯 DEBUG: Final result - Files: {}, Bytes: {}", files_copied, bytes_transferred);
    (files_copied, bytes_transferred)
}

fn parse_robocopy_size(size_str: &str) -> Result<u64, Box<dyn std::error::Error>> {
    println!("🔍 DEBUG: Parsing size: '{}'", size_str);
    
    let size_str = size_str.trim();
    
    // Si es solo un número
    if let Ok(size) = size_str.parse::<u64>() {
        println!("✅ DEBUG: Plain number: {}", size);
        return Ok(size);
    }
    
    // Si tiene sufijo (k, m, g)
    if size_str.len() > 1 {
        let (number_part, suffix) = size_str.split_at(size_str.len() - 1);
        let suffix = suffix.to_lowercase();
        
        println!("🔍 DEBUG: Number part: '{}', suffix: '{}'", number_part, suffix);
        
        if let Ok(number) = number_part.parse::<f64>() {
            let multiplier = match suffix.as_str() {
                "k" => 1024,
                "m" => 1024 * 1024,
                "g" => 1024 * 1024 * 1024,
                "t" => 1024_u64.pow(4),
                _ => return Err(format!("Unknown suffix: {}", suffix).into()),
            };
            
            let result = (number * multiplier as f64) as u64;
            println!("✅ DEBUG: Converted {} {} to {}", number, suffix, result);
            return Ok(result);
        }
    }
    
    Err(format!("Unable to parse size: {}", size_str).into())
}

fn main() {
    let test_output = r#"
-------------------------------------------------------------------------------
   ROBOCOPY     ::     Herramienta para copia eficaz de archivos               
-------------------------------------------------------------------------------

  Inicio: lunes, 18 de agosto de 202522:31:59
   Origen : D:\robocopybackuptoolrust\in1\
     Destino : D:\robocopybackuptoolrust\out1\

    Archivos: *.*

  Opciones: *.* /V /S /E /DCOPY:DA /COPY:DAT /PURGE /MIR /NP /ETA /R:1000000 /W:30 

------------------------------------------------------------------------------

                           2    D:\robocopybackuptoolrust\in1\
                 igual             14836        icoasdasd.png
            Nuevo arch             14836        new_file.png

------------------------------------------------------------------------------

               Total   Copiado   OmitidoNo coincidencia     ERROR    Extras
Director.:         1         0         1         0         0         0
 Archivos:         2         1         1         0         0         0
    Bytes:    28.9 k    14.4 k    14.4 k         0         0         0
   Tiempo:   0:00:00   0:00:00                       0:00:00   0:00:00
   Finalizado: lunes, 18 de agosto de 2025 22:31:59
"#;

    println!("=== TESTING ROBOCOPY PARSER ===");
    let (files, bytes) = parse_robocopy_stats(test_output);
    println!("\n🎯 FINAL RESULT:");
    println!("Files copied: {}", files);
    println!("Bytes transferred: {}", bytes);
}
