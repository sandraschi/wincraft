//! Ready-made color policies: folder depth shading plus WizTree-style file
//! colors by extension.
//!
//! These are conveniences, not requirements. Bring your own palette by
//! ignoring this module entirely; the layout never looks at color.

/// Slate ramp for enveloping folder containers, darkening with depth.
/// Mirrors the WinCraft treemap page.
pub fn depth_color(depth: usize) -> [u8; 3] {
    match depth {
        0 => [15, 23, 42],
        1 => [20, 30, 52],
        2 => [26, 38, 64],
        3 => [32, 46, 76],
        _ => [38, 54, 88],
    }
}

/// File cushion color by lowercase extension (no leading dot).
/// Unknown extensions hash deterministically into a fixed vibrant palette so
/// repeat scans look stable.
pub fn extension_color(ext: &str, is_folder: bool) -> [u8; 3] {
    if is_folder {
        return [71, 85, 105];
    }
    match ext.to_lowercase().as_str() {
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "webm" | "m4v" | "flv" | "m2ts" => [168, 85, 247],
        // Audio
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" => [192, 132, 252],
        // Archives & disk images
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => [249, 115, 22],
        "iso" | "img" | "vmdk" | "vhd" | "vhdx" | "wim" => [239, 68, 68],
        // Executables & binaries
        "exe" | "msi" | "bat" | "cmd" | "ps1" => [34, 197, 94],
        "dll" | "sys" | "so" | "dylib" | "bin" => [16, 185, 129],
        // Documents & web
        "pdf" => [225, 29, 72],
        "doc" | "docx" | "rtf" | "txt" | "md" | "log" => [56, 189, 248],
        "xls" | "xlsx" | "csv" => [13, 148, 136],
        "ppt" | "pptx" => [234, 88, 12],
        // AI / models / big data
        "safetensors" | "ckpt" | "onnx" | "gguf" | "pt" | "pth" => [236, 72, 153],
        "db" | "sqlite" | "sqlite3" | "mdf" | "ldf" | "parquet" => [245, 158, 11],
        // Images
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" | "ico" => [6, 182, 212],
        // Code
        "rs" | "c" | "cpp" | "h" | "hpp" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" => {
            [99, 102, 241]
        }
        // Deterministic hash into a vibrant palette.
        other => {
            let mut h: u32 = 2_166_136_261;
            for b in other.bytes() {
                h = (h ^ (b as u32)).wrapping_mul(16_776_219);
            }
            let palette: &[[u8; 3]] = &[
                [56, 189, 248],
                [168, 85, 247],
                [34, 197, 94],
                [245, 158, 11],
                [244, 63, 94],
                [14, 165, 233],
                [20, 184, 166],
                [234, 179, 8],
                [217, 70, 239],
                [99, 102, 241],
            ];
            palette[(h as usize) % palette.len()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_extensions_map_to_brand_colors() {
        assert_eq!(extension_color("mp4", false), [168, 85, 247]);
        assert_eq!(extension_color("pdf", false), [225, 29, 72]);
        assert_eq!(extension_color("rs", false), [99, 102, 241]);
        assert_eq!(extension_color("anything", true), [71, 85, 105]);
    }

    #[test]
    fn unknown_extensions_are_stable() {
        assert_eq!(
            extension_color("xyzzy", false),
            extension_color("xyzzy", false)
        );
    }

    #[test]
    fn depth_ramp_has_five_stops() {
        assert_eq!(depth_color(0), [15, 23, 42]);
        assert_eq!(depth_color(99), depth_color(4));
    }
}
