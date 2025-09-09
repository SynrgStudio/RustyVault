#!/usr/bin/env pwsh
# ============================================================================
# RustyVault - Script de Release Automatizado
# ============================================================================
# Este script automatiza el proceso completo de release de RustyVault:
# 1. Compila el proyecto en modo release (optimizado)
# 2. Crea un ZIP con el ejecutable y archivos necesarios
# 3. Commitea TODOS los cambios pendientes
# 4. Crea y pushea el tag de versión
# 5. Publica el GitHub Release con el ZIP adjunto
#
# Uso: .\release.ps1 [-Tag] [-Push] [-Clean] [-AutoRelease]

param(
    [switch]$Tag,         # Crear tag git automáticamente
    [switch]$Push,        # Hacer push al repositorio remoto
    [switch]$Clean,       # Limpiar archivos temporales al final
    [switch]$AutoRelease  # Crear release automático en GitHub
)

# Configuración del proyecto
$PROJECT_NAME = "rusty-vault"
$RELEASE_NAME = "RustyVault"

# Archivos y carpetas a incluir en el release
$INCLUDE_PATHS = @(
    "target/release/rusty-vault.exe",
    "README.md",
    "LICENSE.txt",
    "ico.ico"
)

# Archivos y carpetas a excluir del repositorio
$EXCLUDE_FROM_REPO = @(
    "target/",
    "*.pdb",
    "*.log",
    "test_*",
    "build_release.ps1"  # El archivo viejo que queremos eliminar
)

Write-Host ">> RustyVault Release Builder" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan

# Función para extraer versión del Cargo.toml
function Get-ProjectVersion {
    try {
        $cargoContent = Get-Content "Cargo.toml" -Raw
        if ($cargoContent -match 'version\s*=\s*"([^"]+)"') {
            return $matches[1]
        }
        throw "No se pudo extraer la versión"
    }
    catch {
        Write-Error "ERROR: Error al leer version del Cargo.toml: $_"
        exit 1
    }
}

# Función para verificar que estamos en un repositorio git
function Test-GitStatus {
    try {
        $status = git status --porcelain 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "WARNING: No se detectó repositorio git"
            return $false
        }
        
        if ($status) {
            Write-Warning "WARNING: Hay cambios sin commitear:"
            git status --short
            $continue = Read-Host "¿Continuar de todas formas? (y/N)"
            return ($continue -eq "y" -or $continue -eq "Y")
        }
        return $true
    }
    catch {
        Write-Warning "WARNING: Error al verificar estado de git: $_"
        return $false
    }
}

# Función para compilar el proyecto
function Invoke-CargoBuild {
    try {
        Write-Host "  -> Compilando RustyVault en modo release..." -ForegroundColor Green
        
        # Limpiar build anterior
        cargo clean
        
        # Compilar en modo release
        cargo build --release
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  -> Compilación exitosa!" -ForegroundColor Green
            
            # Verificar que el ejecutable existe
            if (Test-Path "target/release/rusty-vault.exe") {
                $exeSize = (Get-Item "target/release/rusty-vault.exe").Length / 1MB
                Write-Host "  -> Ejecutable creado: rusty-vault.exe ($([math]::Round($exeSize, 2)) MB)" -ForegroundColor Green
                return $true
            } else {
                Write-Error "ERROR: No se encontró el ejecutable compilado"
                return $false
            }
        } else {
            Write-Error "ERROR: Fallo en la compilación"
            return $false
        }
    }
    catch {
        Write-Error "ERROR: Error durante la compilación: $_"
        return $false
    }
}

# Función para crear el release zip
function New-ReleaseZip {
    param([string]$Version)
    
    $releaseZip = "$PROJECT_NAME-v$Version-windows.zip"
    $tempDir = "temp_release"
    $releaseDir = Join-Path $tempDir $RELEASE_NAME
    
    Write-Host ">> Creando release v$Version..." -ForegroundColor Green
    
    try {
        # Limpiar y crear directorio temporal
        if (Test-Path $tempDir) {
            Remove-Item $tempDir -Recurse -Force
        }
        New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null
        
        # Copiar archivos incluidos
        foreach ($path in $INCLUDE_PATHS) {
            if (Test-Path $path) {
                $fileName = Split-Path $path -Leaf
                $dest = Join-Path $releaseDir $fileName
                Write-Host "  -> Copiando $path" -ForegroundColor Yellow
                Copy-Item $path $dest -Force
            } else {
                Write-Warning "  -> Archivo no encontrado: $path"
            }
        }
        
        # Crear archivo de información de versión
        $versionInfo = @"
RustyVault v$Version
====================

Release Date: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Platform: Windows x64
Build: Release (Optimized)

Installation:
1. Extract all files to a folder
2. Run rusty-vault.exe
3. Configure your backup settings

For more information, visit:
https://github.com/SynrgStudio/RustyVault
"@
        
        $versionFile = Join-Path $releaseDir "VERSION.txt"
        $versionInfo | Out-File -FilePath $versionFile -Encoding UTF8
        Write-Host "  -> Creado VERSION.txt" -ForegroundColor Yellow
        
        # Crear el zip
        Write-Host "  -> Creando $releaseZip..." -ForegroundColor Green
        if (Test-Path $releaseZip) {
            Remove-Item $releaseZip -Force
        }
        
        Compress-Archive -Path (Join-Path $tempDir "*") -DestinationPath $releaseZip -Force
        
        # Mostrar información del zip creado
        $zipSize = (Get-Item $releaseZip).Length / 1MB
        Write-Host "  -> Release creado: $releaseZip ($([math]::Round($zipSize, 2)) MB)" -ForegroundColor Green
        
        # Listar contenido para verificación
        Write-Host "  -> Contenido del release:" -ForegroundColor Cyan
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path $releaseZip))
        $zip.Entries | ForEach-Object { 
            Write-Host "   $($_.FullName)" -ForegroundColor Gray
        }
        $zip.Dispose()
        
        return $releaseZip
    }
    catch {
        Write-Error "ERROR: Error al crear release: $_"
        return $null
    }
    finally {
        # Limpiar directorio temporal si se solicita
        if ($Clean -and (Test-Path $tempDir)) {
            Remove-Item $tempDir -Recurse -Force
            Write-Host "  -> Archivos temporales limpiados" -ForegroundColor Gray
        }
    }
}

# Función para limpiar archivos del repositorio
function Remove-ExcludedFiles {
    Write-Host "  -> Limpiando archivos excluidos del repositorio..." -ForegroundColor Green
    
    foreach ($pattern in $EXCLUDE_FROM_REPO) {
        try {
            if ($pattern -eq "target/") {
                # Caso especial para target/ - usar git rm si existe en el índice
                $targetFiles = git ls-files target/ 2>$null
                if ($targetFiles) {
                    Write-Host "    -> Removiendo target/ del repositorio..." -ForegroundColor Yellow
                    git rm -r --cached target/ 2>$null
                }
            }
            elseif ($pattern -eq "build_release.ps1") {
                # Remover el archivo viejo específicamente
                if (git ls-files $pattern 2>$null) {
                    Write-Host "    -> Removiendo $pattern del repositorio..." -ForegroundColor Yellow
                    git rm --cached $pattern 2>$null
                }
            }
            else {
                # Para otros patrones, usar git rm con wildcards
                $files = git ls-files $pattern 2>$null
                if ($files) {
                    Write-Host "    -> Removiendo archivos que coinciden con $pattern..." -ForegroundColor Yellow
                    git rm --cached $pattern 2>$null
                }
            }
        }
        catch {
            Write-Host "    -> No se pudo remover $pattern (puede que no exista)" -ForegroundColor Gray
        }
    }
}

# Función para crear tag git
function New-GitTag {
    param([string]$Version)
    
    try {
        $tagName = "v$Version"
        Write-Host "  -> Creando tag $tagName..." -ForegroundColor Green
        
        git tag -a $tagName -m "Release $tagName" 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  -> Tag $tagName creado" -ForegroundColor Green
            
            if ($Push) {
                Write-Host "  -> Haciendo push del tag..." -ForegroundColor Green
                git push origin $tagName
                if ($LASTEXITCODE -eq 0) {
                    Write-Host "  -> Tag enviado al repositorio remoto" -ForegroundColor Green
                    return $true
                } else {
                    Write-Warning "  -> Error al enviar tag al repositorio remoto"
                    return $false
                }
            }
            return $true
        } else {
            Write-Warning "  -> Error al crear tag (puede que ya exista)"
            return $false
        }
    }
    catch {
        Write-Warning "  -> Error con git tag: $_"
        return $false
    }
}

# Función para commit y push de todos los cambios
function Invoke-GitCommitAndPush {
    param([string]$ReleaseZip, [string]$Version)
    
    try {
        Write-Host "  -> Agregando cambios al repositorio..." -ForegroundColor Green
        
        # Primero limpiar archivos excluidos
        Remove-ExcludedFiles
        
        # Agregar todos los cambios (excluyendo lo que está en .gitignore)
        git add .
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "  -> Error al agregar archivos a git"
            return $false
        }
        
        # Commit con mensaje descriptivo
        $commitMessage = "Release v$Version - RustyVault Windows build"
        git commit -m $commitMessage
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  -> Commit creado: $commitMessage" -ForegroundColor Green
            
            if ($Push) {
                Write-Host "  -> Haciendo push de commits..." -ForegroundColor Green
                git push
                if ($LASTEXITCODE -eq 0) {
                    Write-Host "  -> Commits enviados al repositorio remoto" -ForegroundColor Green
                    return $true
                } else {
                    Write-Warning "  -> Error al enviar commits al repositorio remoto"
                    return $false
                }
            }
            return $true
        } else {
            Write-Host "  -> Sin cambios para commitear" -ForegroundColor Yellow
            return $true
        }
    }
    catch {
        Write-Warning "  -> Error en commit/push: $_"
        return $false
    }
}

# Función para crear GitHub Release
function New-GitHubRelease {
    param([string]$Version, [string]$ReleaseZip)
    
    try {
        $tagName = "v$Version"
        Write-Host "  -> Creando GitHub Release $tagName..." -ForegroundColor Green
        
        # Verificar que GitHub CLI está disponible
        $ghVersion = gh --version 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "  -> GitHub CLI no está instalado o configurado"
            Write-Host "  -> Instala GitHub CLI desde: https://cli.github.com/" -ForegroundColor Yellow
            return $false
        }
        
        # Crear el release con el ZIP adjunto
        $releaseTitle = "RustyVault v$Version"
        $releaseNotes = @"
## RustyVault v$Version

### 🚀 Release Notes
- Modern backup automation tool built with Rust
- Intuitive GUI with system tray integration
- High-performance file operations
- Windows-optimized build

### 📦 Installation
1. Download the ``$ReleaseZip`` file below
2. Extract all files to a folder of your choice
3. Run ``rusty-vault.exe``
4. Configure your backup settings through the GUI

### ✨ What's Included
- Optimized Windows executable (Release build)
- Application icon and resources
- Documentation and license
- Ready-to-run package

### 🔧 System Requirements
- Windows 10/11 (x64)
- No additional dependencies required

For detailed documentation and source code, visit our [GitHub repository](https://github.com/SynrgStudio/RustyVault).
"@
        
        gh release create $tagName $ReleaseZip --title $releaseTitle --notes $releaseNotes
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  -> GitHub Release creado exitosamente!" -ForegroundColor Green
            
            # Obtener URL del repositorio
            $repoUrl = git config --get remote.origin.url 2>$null
            if ($repoUrl) {
                $repoUrl = $repoUrl -replace '\.git$', '' -replace 'git@github\.com:', 'https://github.com/'
                Write-Host "  -> URL: $repoUrl/releases/tag/$tagName" -ForegroundColor Cyan
            }
            return $true
        } else {
            Write-Warning "  -> Error al crear GitHub Release"
            return $false
        }
    }
    catch {
        Write-Warning "  -> Error al crear GitHub Release: $_"
        return $false
    }
}

# ============================================================================
# MAIN SCRIPT
# ============================================================================

# Verificar que estamos en el directorio correcto
if (-not (Test-Path "Cargo.toml")) {
    Write-Error "ERROR: No se encontró Cargo.toml. Ejecuta este script desde la raíz del proyecto Rust."
    exit 1
}

# Verificar que Rust está instalado
try {
    $rustVersion = cargo --version 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Error "ERROR: Rust/Cargo no está instalado. Instala desde https://rustup.rs/"
        exit 1
    }
    Write-Host "  -> Rust detectado: $rustVersion" -ForegroundColor Gray
}
catch {
    Write-Error "ERROR: No se pudo verificar instalación de Rust"
    exit 1
}

# Obtener versión
$version = Get-ProjectVersion
Write-Host "  -> Versión detectada: v$version" -ForegroundColor Cyan

# AutoRelease activa automáticamente Tag y Push
if ($AutoRelease) {
    $Tag = $true
    $Push = $true
    Write-Host "  -> AutoRelease activado (Tag y Push automáticos)" -ForegroundColor Cyan
}

# Verificar estado git (solo si NO es AutoRelease)
if ($Tag -and -not $AutoRelease -and -not (Test-GitStatus)) {
    Write-Host "ERROR: Estado de git no es válido para crear tag" -ForegroundColor Red
    exit 1
}

# Autorelease completo
if ($AutoRelease) {
    Write-Host "`n>> Iniciando AutoRelease completo..." -ForegroundColor Cyan
    
    # 1. Compilar proyecto
    $buildSuccess = Invoke-CargoBuild
    if (-not $buildSuccess) {
        Write-Error "ERROR: Fallo en la compilación. Abortando AutoRelease."
        exit 1
    }
    
    # 2. Crear release ZIP
    $releaseZip = New-ReleaseZip -Version $version
    if (-not $releaseZip) {
        Write-Error "ERROR: No se pudo crear ZIP de release."
        exit 1
    }
    
    # 3. Commit y push de todos los cambios
    $commitSuccess = Invoke-GitCommitAndPush -ReleaseZip $releaseZip -Version $version
    if (-not $commitSuccess) {
        Write-Warning "Fallo en commit/push, continuando con tag..."
    }
    
    # 4. Crear y push tag
    $tagSuccess = New-GitTag -Version $version
    if (-not $tagSuccess) {
        Write-Error "ERROR: No se pudo crear/push tag. Abortando AutoRelease."
        exit 1
    }
    
    # 5. Crear GitHub Release
    $releaseSuccess = New-GitHubRelease -Version $version -ReleaseZip $releaseZip
    if (-not $releaseSuccess) {
        Write-Error "ERROR: No se pudo crear GitHub Release."
        exit 1
    }
}
else {
    # Compilar y crear release ZIP para otros modos
    $buildSuccess = Invoke-CargoBuild
    if (-not $buildSuccess) {
        exit 1
    }
    
    $releaseZip = New-ReleaseZip -Version $version
    if (-not $releaseZip) {
        exit 1
    }
    
    # Crear tag solamente si se solicita (y no es AutoRelease)
    if ($Tag) {
        New-GitTag -Version $version
    }
}

Write-Host "`n  -> Release completado exitosamente!" -ForegroundColor Green
Write-Host "   Archivo: $releaseZip" -ForegroundColor White
Write-Host "   Version: v$version" -ForegroundColor White

if ($AutoRelease) {
    Write-Host "   Tag: v$version (creado y pusheado)" -ForegroundColor Green
    Write-Host "   GitHub Release: Creado automaticamente" -ForegroundColor Green
    Write-Host "   Estado: LISTO PARA DISTRIBUCION" -ForegroundColor Green
    
    Write-Host "`n  -> AutoRelease completado!" -ForegroundColor Cyan
    Write-Host "   1. Proyecto compilado en modo release" -ForegroundColor Gray
    Write-Host "   2. Todos los cambios commiteados al repositorio" -ForegroundColor Gray
    Write-Host "   3. Tag v$version creado y pusheado" -ForegroundColor Gray  
    Write-Host "   4. GitHub Release publicado con ZIP adjunto" -ForegroundColor Gray
    Write-Host "   5. RustyVault listo para descarga publica" -ForegroundColor Gray
}
elseif ($Tag) {
    Write-Host "   Tag: v$version" -ForegroundColor White
    
    Write-Host "`n  -> Proximos pasos:" -ForegroundColor Yellow
    Write-Host "   1. Probar el ejecutable desde $releaseZip" -ForegroundColor Gray
    Write-Host "   2. Subir a GitHub Releases manualmente" -ForegroundColor Gray
}
else {
    Write-Host "`n  -> Proximos pasos:" -ForegroundColor Yellow
    Write-Host "   1. Probar el ejecutable desde $releaseZip" -ForegroundColor Gray
    Write-Host "   2. Crear release completo con: .\release.ps1 -AutoRelease" -ForegroundColor Gray
    Write-Host "   3. O solo tag con: .\release.ps1 -Tag" -ForegroundColor Gray
}