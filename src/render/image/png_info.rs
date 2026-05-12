//! PNG IHDR 청크에서 width/height 직접 추출 (의존성 0).

const SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// PNG bytes에서 (width, height)를 반환. 시그니처/IHDR 검증 실패 시 None.
pub fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 { return None; }
    if &bytes[..8] != SIGNATURE { return None; }
    // 청크 헤더: 4 bytes length, 4 bytes type
    let chunk_type = &bytes[12..16];
    if chunk_type != b"IHDR" { return None; }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    // 1x1 red pixel PNG (well-known)
    const ONE_PIXEL: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    fn one_pixel_bytes() -> Vec<u8> {
        base64::engine::general_purpose::STANDARD.decode(ONE_PIXEL).unwrap()
    }

    #[test]
    fn reads_1x1_dimensions() {
        let b = one_pixel_bytes();
        assert_eq!(dimensions(&b), Some((1, 1)));
    }

    #[test]
    fn rejects_non_png_signature() {
        let b = b"not-a-png-file";
        assert_eq!(dimensions(b), None);
    }

    #[test]
    fn rejects_too_short_bytes() {
        let b = b"\x89PNG";
        assert_eq!(dimensions(b), None);
    }

    #[test]
    fn rejects_missing_ihdr() {
        // 시그니처는 OK지만 IHDR 청크 타입 아님
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        b.extend_from_slice(&[0, 0, 0, 13]); // chunk length
        b.extend_from_slice(b"XXXX"); // wrong type
        b.extend_from_slice(&[0u8; 13]); // dummy data
        b.extend_from_slice(&[0u8; 4]); // crc
        assert_eq!(dimensions(&b), None);
    }
}
