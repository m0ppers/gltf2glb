use base64;
use serde_json;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

#[repr(C)]
struct GlbHeader {
    magic: u32,
    version: u32,
    length: u32,
}

impl GlbHeader {
    fn new(len: u32) -> GlbHeader {
        GlbHeader {
            magic: 0x46546C67,
            version: 2,
            length: len,
        }
    }
}

#[repr(C)]
struct GlbChunk {
    chunk_length: u32,
    chunk_type: u32,
}

impl GlbChunk {
    fn json(len: u32) -> GlbChunk {
        GlbChunk {
            chunk_length: len,
            chunk_type: 0x4E4F534A,
        }
    }

    fn binary(len: u32) -> GlbChunk {
        GlbChunk {
            chunk_length: len,
            chunk_type: 0x004E4942,
        }
    }
}

// stackoverflow copy pasta
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

fn check_is_gltf(json: &serde_json::Value) -> Result<(), String> {
    if json.pointer("/asset/version") != Some(&"2.0".into()) {
        return Err("File doesn't seem to be a glTF file".into());
    }
    Ok(())
}

fn prepare_chunks(
    mut json: serde_json::Value,
) -> Result<(serde_json::Value, Option<Vec<u8>>), String> {
    let buffers = match json["buffers"].as_array_mut() {
        Some(buffers) => buffers,
        _ => return Ok((json, None)),
    };

    let buffer = buffers
        .into_iter()
        .try_fold(vec![], |mut v: Vec<u8>, buffer| {
            let buffer_object = buffer
                .as_object_mut()
                .ok_or(String::from("Buffer is not an object"))?;

            // leaves null in place
            let uri = buffer_object["uri"].take();
            // also kill key
            buffer_object.remove("uri");
            let uri_str = uri
                .as_str()
                .ok_or(String::from("Buffer URI is not a string"))?;
            if uri_str.len() < 37 {
                return Err(format!("Expected base64 uri but got {}", uri));
            }
            let (mime, base64data) = uri_str.split_at(37);
            if mime != "data:application/octet-stream;base64," {
                return Err(format!("Invalid mimetype {}", mime));
            }
            let data = match base64::decode(base64data) {
                Ok(data) => data,
                Err(_) => return Err("Couldn't decode base64".into()),
            };
            v.extend(data.iter());
            Ok(v)
        })?;
    Ok((json, Some(buffer)))
}

fn padded<W, F, E: 'static, R>(writer: &mut W, pad: &[u8; 1], f: F) -> Result<u64, Box<dyn Error>>
where
    W: Write + Seek,
    F: FnOnce(&mut W) -> Result<R, E>,
    E: Error,
{
    let start = writer.seek(SeekFrom::Current(0))?;
    f(writer)?;
    let end = writer.seek(SeekFrom::Current(0))?;

    let mut size = end - start;

    let padding = (4 - size % 4) % 4;
    if padding > 0 {
        size = size + padding;
        writer.write(&pad.repeat(padding as usize))?;
    }

    Ok(size)
}

pub fn convert<P>(input: P, output: P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let infile = File::open(input)?;
    let reader = BufReader::new(infile);
    let json = serde_json::from_reader(reader)?;
    check_is_gltf(&json)?;

    let outfile = File::create(output)?;
    let mut writer = BufWriter::new(outfile);

    let json_offset = (std::mem::size_of::<GlbHeader>() + std::mem::size_of::<GlbChunk>()) as u64;
    writer.seek(SeekFrom::Start(json_offset))?;

    let (json, buffer) = prepare_chunks(json)?;

    let json_size = padded(&mut writer, b" ", |writer| {
        serde_json::to_writer(writer, &json)
    })?;

    let total_size = match buffer {
        Some(buffer) => {
            let padding = (4 - buffer.len() % 4) % 4;
            let binary_chunk = GlbChunk::binary(buffer.len() as u32);
            unsafe {
                writer.write(any_as_u8_slice(&binary_chunk))?;
            }
            padded(&mut writer, &[0], |writer| writer.write(&buffer))?;
            json_size
                + json_offset
                + std::mem::size_of::<GlbChunk>() as u64
                + (buffer.len() + padding) as u64
        }
        None => json_size + json_offset,
    };

    writer.seek(SeekFrom::Start(0))?;
    let header = GlbHeader::new(total_size as u32);
    let json_chunk = GlbChunk::json(json_size as u32);
    unsafe {
        writer.write(any_as_u8_slice(&header))?;
        writer.write(any_as_u8_slice(&json_chunk))?;
    }

    Ok(())
}
