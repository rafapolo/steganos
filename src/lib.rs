use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use flate2::read::ZlibDecoder;
use png::{ColorType, Decoder, Encoder, Transformations};
use rayon::prelude::*;
use thiserror::Error;
use zstd::stream::decode_all as zstd_decode_all;

const AUTHOR: &str = "ExtraPolo!";
const PAYLOAD_LENGTH_KEY: &str = "PayloadLength";
const ZSTD_LEVEL: i32 = 3;

#[derive(Debug, Error)]
pub enum SteganosError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("png decode error: {0}")]
    PngDecode(#[from] png::DecodingError),
    #[error("png encode error: {0}")]
    PngEncode(#[from] png::EncodingError),
    #[error("invalid hex: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("invalid base64: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid utf-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("invalid payload length")]
    InvalidPayloadLength,
    #[error("missing or unsupported PNG color type")]
    UnsupportedColorType,
}

#[derive(Debug, Clone)]
pub struct EncodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels_rgb: Vec<u8>,
    pub payload_len: usize,
}

pub fn encode_file_to_png(input: &Path, output: Option<&Path>) -> Result<PathBuf, SteganosError> {
    let title = input.to_string_lossy();
    let out_path = match output {
        Some(path) => path.to_path_buf(),
        None => PathBuf::from(format!("{}.png", title)),
    };

    let image = encode_file_to_image_streaming(input)?;
    write_png(&out_path, &image, &title, AUTHOR, image.payload_len)?;
    Ok(out_path)
}

pub fn decode_png_to_file(input: &Path, output: Option<&Path>) -> Result<PathBuf, SteganosError> {
    let file = File::open(input)?;
    let reader = BufReader::new(file);
    let mut decoder = Decoder::new(reader);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    let bytes = &buf[..info.buffer_size()];

    let pixels_rgb = match info.color_type {
        ColorType::Rgb => bytes.to_vec(),
        ColorType::Rgba => rgba_to_rgb(bytes),
        ColorType::Grayscale => gray_to_rgb(bytes),
        ColorType::GrayscaleAlpha => gray_alpha_to_rgb(bytes),
        _ => return Err(SteganosError::UnsupportedColorType),
    };

    let title = extract_text_chunk(&reader.info(), "Title").unwrap_or_default();
    let payload_len = extract_text_chunk(&reader.info(), PAYLOAD_LENGTH_KEY)
        .and_then(|v| v.parse::<usize>().ok());

    let out_path = match output {
        Some(path) => path.to_path_buf(),
        None => PathBuf::from(format!("out-{}", title)),
    };

    if let Some(payload_len) = payload_len {
        let payload_reader = PayloadReader::new(pixels_rgb, payload_len);
        let mut decoder = zstd::stream::Decoder::new(payload_reader)?;
        let file = File::create(&out_path)?;
        let mut writer = TrackingWriter::new(BufWriter::new(file));
        std::io::copy(&mut decoder, &mut writer)?;
        if !writer.ends_with_newline() {
            writer.write_all(b"\n")?;
        }
        return Ok(out_path);
    }

    let data = decode_legacy_pixels(pixels_rgb, info.width as usize, info.height as usize)?;
    write_compat_output(&out_path, &data)?;
    Ok(out_path)
}


pub fn encode_bytes_to_image(data: &[u8]) -> EncodedImage {
    let zipped = zstd_deflate(data);
    pack_payload_to_image(&zipped)
}

fn encode_file_to_image_streaming(input: &Path) -> Result<EncodedImage, SteganosError> {
    let temp_path = temp_path();
    let temp_guard = TempPath::new(temp_path.clone());

    let input_file = File::open(input)?;
    let mut input_reader = BufReader::new(input_file);
    let temp_file = File::create(&temp_path)?;
    let mut encoder = zstd::stream::Encoder::new(temp_file, ZSTD_LEVEL)?;
    set_zstd_multithread(&mut encoder);
    std::io::copy(&mut input_reader, &mut encoder)?;
    let temp_file = encoder.finish()?;
    let payload_len = temp_file.metadata()?.len() as usize;

    let image = pack_payload_from_file(&temp_path, payload_len)?;
    drop(temp_guard);
    Ok(image)
}

fn pack_payload_from_file(path: &Path, payload_len: usize) -> Result<EncodedImage, SteganosError> {
    if payload_len == 0 {
        return Ok(EncodedImage {
            width: 1,
            height: 1,
            pixels_rgb: vec![0u8; 3],
            payload_len,
        });
    }
    let size_pixels = (payload_len + 2) / 3;
    let width = (size_pixels as f64).sqrt().ceil() as u32;
    let height = ((size_pixels as u32) + width - 1) / width;

    let mut pixels_rgb = vec![0u8; (width * height * 3) as usize];
    let mut reader = BufReader::new(File::open(path)?);
    let mut offset = 0usize;
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let end = offset + read;
        pixels_rgb[offset..end].copy_from_slice(&buffer[..read]);
        offset = end;
    }
    if offset != payload_len {
        return Err(SteganosError::InvalidPayloadLength);
    }

    Ok(EncodedImage {
        width,
        height,
        pixels_rgb,
        payload_len,
    })
}

fn pack_payload_to_image(payload: &[u8]) -> EncodedImage {
    let payload_len = payload.len();
    if payload_len == 0 {
        return EncodedImage {
            width: 1,
            height: 1,
            pixels_rgb: vec![0u8; 3],
            payload_len,
        };
    }
    let size_pixels = (payload_len + 2) / 3;
    let width = (size_pixels as f64).sqrt().ceil() as u32;
    let height = ((size_pixels as u32) + width - 1) / width;

    let mut pixels_rgb = vec![0u8; (width * height * 3) as usize];
    pixels_rgb
        .par_chunks_mut(3)
        .enumerate()
        .for_each(|(i, px)| {
            let base = i * 3;
            if base >= payload_len {
                return;
            }
            px[0] = payload[base];
            px[1] = if base + 1 < payload_len { payload[base + 1] } else { 0 };
            px[2] = if base + 2 < payload_len { payload[base + 2] } else { 0 };
        });

    EncodedImage {
        width,
        height,
        pixels_rgb,
        payload_len,
    }
}

pub fn decode_png_to_bytes(path: &Path) -> Result<(Vec<u8>, Option<String>), SteganosError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut decoder = Decoder::new(reader);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    let bytes = &buf[..info.buffer_size()];

    let (width, height) = (info.width as usize, info.height as usize);
    let pixels_rgb = match info.color_type {
        ColorType::Rgb => bytes.to_vec(),
        ColorType::Rgba => rgba_to_rgb(bytes),
        ColorType::Grayscale => gray_to_rgb(bytes),
        ColorType::GrayscaleAlpha => gray_alpha_to_rgb(bytes),
        _ => return Err(SteganosError::UnsupportedColorType),
    };

    let title = extract_text_chunk(&reader.info(), "Title");
    let payload_len = extract_text_chunk(&reader.info(), PAYLOAD_LENGTH_KEY)
        .and_then(|v| v.parse::<usize>().ok());

    if let Some(payload_len) = payload_len {
        let payload = extract_payload_from_rgb(&pixels_rgb, payload_len)?;
        let data = zstd_inflate(&payload)?;
        return Ok((data, title));
    }

    let data = decode_legacy_pixels(pixels_rgb, width, height)?;
    Ok((data, title))
}

pub fn write_png(
    path: &Path,
    image: &EncodedImage,
    title: &str,
    author: &str,
    payload_len: usize,
) -> Result<(), SteganosError> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut encoder = Encoder::new(writer, image.width, image.height);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Best);
    encoder.add_text_chunk("Author".to_string(), author.to_string())?;
    encoder.add_text_chunk("Title".to_string(), title.to_string())?;
    encoder.add_text_chunk(PAYLOAD_LENGTH_KEY.to_string(), payload_len.to_string())?;

    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(&image.pixels_rgb)?;
    Ok(())
}


pub fn write_compat_output(path: &Path, data: &[u8]) -> Result<(), SteganosError> {
    let mut file = File::create(path)?;
    file.write_all(data)?;
    if !data.ends_with(b"\n") {
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn zlib_inflate(data: &[u8]) -> Result<Vec<u8>, SteganosError> {
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

fn zstd_deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), ZSTD_LEVEL).expect("zstd encoder");
    set_zstd_multithread(&mut encoder);
    encoder.write_all(data).expect("zstd write");
    encoder.finish().expect("zstd finish")
}

fn zstd_inflate(data: &[u8]) -> Result<Vec<u8>, SteganosError> {
    Ok(zstd_decode_all(data)?)
}

fn extract_payload_from_rgb(pixels: &[u8], payload_len: usize) -> Result<Vec<u8>, SteganosError> {
    if payload_len == 0 {
        return Ok(Vec::new());
    }
    let mut payload = vec![0u8; payload_len];
    payload
        .par_chunks_mut(3)
        .enumerate()
        .for_each(|(i, chunk)| {
            let pixel_index = i * 3;
            if pixel_index + 2 >= pixels.len() {
                return;
            }
            let r = pixels[pixel_index];
            let g = pixels[pixel_index + 1];
            let b = pixels[pixel_index + 2];
            chunk[0] = r;
            if chunk.len() > 1 {
                chunk[1] = g;
            }
            if chunk.len() > 2 {
                chunk[2] = b;
            }
        });
    Ok(payload)
}

fn set_zstd_multithread<W: Write>(encoder: &mut zstd::stream::Encoder<'_, W>) {
    if let Ok(threads) = std::thread::available_parallelism() {
        let n = threads.get().min(u32::MAX as usize) as u32;
        let _ = encoder.multithread(n);
    }
}

fn temp_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = format!("steganos-{}-{}.tmp", std::process::id(), nanos);
    path.push(name);
    path
}

struct TempPath {
    path: PathBuf,
}

impl TempPath {
    fn new(path: PathBuf) -> Self {
        TempPath { path }
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

struct PayloadReader {
    pixels: Vec<u8>,
    pos: usize,
    len: usize,
}

impl PayloadReader {
    fn new(pixels: Vec<u8>, len: usize) -> Self {
        PayloadReader { pixels, pos: 0, len }
    }
}

impl Read for PayloadReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.len {
            return Ok(0);
        }
        let remaining = self.len - self.pos;
        let to_copy = remaining.min(buf.len());
        buf[..to_copy].copy_from_slice(&self.pixels[self.pos..self.pos + to_copy]);
        self.pos += to_copy;
        Ok(to_copy)
    }
}

struct TrackingWriter<W: Write> {
    inner: W,
    last: Option<u8>,
}

impl<W: Write> TrackingWriter<W> {
    fn new(inner: W) -> Self {
        TrackingWriter { inner, last: None }
    }

    fn ends_with_newline(&self) -> bool {
        matches!(self.last, Some(b'\n'))
    }
}

impl<W: Write> Write for TrackingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        if written > 0 {
            self.last = Some(buf[written - 1]);
        }
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn rgba_to_rgb(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity((rgba.len() / 4) * 3);
    for chunk in rgba.chunks(4) {
        out.push(chunk[0]);
        out.push(chunk[1]);
        out.push(chunk[2]);
    }
    out
}

fn gray_to_rgb(gray: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(gray.len() * 3);
    for &v in gray {
        out.push(v);
        out.push(v);
        out.push(v);
    }
    out
}

fn gray_alpha_to_rgb(gray_alpha: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity((gray_alpha.len() / 2) * 3);
    for chunk in gray_alpha.chunks(2) {
        let v = chunk[0];
        out.push(v);
        out.push(v);
        out.push(v);
    }
    out
}

fn extract_text_chunk(info: &png::Info, key: &str) -> Option<String> {
    for t in &info.utf8_text {
        if t.keyword == key {
            return Some(t.get_text().ok()?);
        }
    }
    for t in &info.uncompressed_latin1_text {
        if t.keyword == key {
            return Some(t.text.clone());
        }
    }
    for t in &info.compressed_latin1_text {
        if t.keyword == key {
            return Some(t.get_text().ok()?);
        }
    }
    None
}

fn decode_legacy_pixels(pixels: Vec<u8>, width: usize, height: usize) -> Result<Vec<u8>, SteganosError> {
    let mut hex_encoded = String::new();
    let size = width * height;
    let mut count = 0usize;

    for i in (0..pixels.len()).step_by(3) {
        count += 1;
        let r = pixels[i];
        let g = pixels[i + 1];
        let b = pixels[i + 2];

        let mut chunk = format!("{:02x}{:02x}{:02x}", r, g, b);
        if count == size {
            while chunk.ends_with('0') {
                chunk.pop();
            }
        }
        hex_encoded.push_str(&chunk);
    }

    let zipped = hex::decode(hex_encoded)?;
    let b64 = zlib_inflate(&zipped)?;
    let b64_str = String::from_utf8(b64)?;
    let data = base64_decode64(&b64_str)?;
    Ok(data)
}

fn base64_decode64(input: &str) -> Result<Vec<u8>, SteganosError> {
    let cleaned: String = input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let decoded = base64::engine::general_purpose::STANDARD.decode(cleaned)?;
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_decode_roundtrip() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("cypherpunks.pdf.png");
        let (data, _title) = decode_png_to_bytes(&path).expect("decode png");
        assert!(!data.is_empty());
        let encoded = encode_bytes_to_image(&data);
        let out_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tmp-roundtrip.png");
        write_png(&out_path, &encoded, "tmp", AUTHOR, encoded.payload_len).expect("write png");
        let (roundtrip, _title) = decode_png_to_bytes(&out_path).expect("decode png");
        assert_eq!(data, roundtrip);
        let _ = std::fs::remove_file(out_path);
    }

}
