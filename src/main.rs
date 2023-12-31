#![allow(dead_code, unused_variables)] 

// use raylib::prelude::*;
use anyhow::{ Result, ensure };
use std::{io::{Read, Seek, SeekFrom}, fs::File, slice::Iter, fmt, collections::HashMap};
use flate2::bufread::ZlibDecoder;

fn chunk_loc_to_byte_offset(bytes: [u8; 4]) -> Option<u64> {
    if bytes[3] == 0 {
        None
    } else {
        Some((((bytes[0] as u64) << 16) + ((bytes[1] as u64) << 8) + ((bytes[2] as u64) << 0)) * 4096)
    }
}

trait NextPlusPlus {
    fn next_n_vec(&mut self, n: usize) -> Option<Vec<u8>>;

    fn next_n<const N: usize>(&mut self) -> Option<[u8; N]>;

    fn next_n_i8_vec(&mut self, n: usize) -> Option<Vec<i8>>;
    fn next_n_i32_vec(&mut self, n: usize) -> Option<Vec<i32>>;
    fn next_n_i64_vec(&mut self, n: usize) -> Option<Vec<i64>>;

    fn next_u8(&mut self) -> Option<u8>;
    fn next_u16(&mut self) -> Option<u16>;
    fn next_u32(&mut self) -> Option<u32>;
    fn next_u64(&mut self) -> Option<u64>;
    
    fn next_i8(&mut self) -> Option<i8>;
    fn next_i16(&mut self) -> Option<i16>;
    fn next_i32(&mut self) -> Option<i32>;
    fn next_i64(&mut self) -> Option<i64>;

    fn next_f32(&mut self) -> Option<f32>;
    fn next_f64(&mut self) -> Option<f64>;

    fn next_string(&mut self, len: usize) -> Option<String>;

}

impl NextPlusPlus for Iter<'_, u8> {

    fn next_n_vec(&mut self, n: usize) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        for _ in 0..n {
            bytes.push(*self.next()?);
        }
        Some(bytes)
    }

    fn next_n<const N: usize>(&mut self) -> Option<[u8; N]> {
        let mut bytes = [0; N];
        for i in 0..N {
            bytes[i] = *self.next()?;
        }
        Some(bytes)
    }
    
    fn next_u8(&mut self) -> Option<u8> {
        Some(u8::from_be_bytes(self.next_n::<1>()?))
    }

    fn next_u16(&mut self) -> Option<u16> {
        Some(u16::from_be_bytes(self.next_n::<2>()?))
    }

    fn next_u32(&mut self) -> Option<u32> {
        Some(u32::from_be_bytes(self.next_n::<4>()?))
    }

    fn next_u64(&mut self) -> Option<u64> {
        Some(u64::from_be_bytes(self.next_n::<8>()?))
    }

    fn next_i8(&mut self) -> Option<i8> {
        Some(i8::from_be_bytes(self.next_n::<1>()?))
    }

    fn next_i16(&mut self) -> Option<i16> {
        Some(i16::from_be_bytes(self.next_n::<2>()?))
    }

    fn next_i32(&mut self) -> Option<i32> {
        Some(i32::from_be_bytes(self.next_n::<4>()?))
    }

    fn next_i64(&mut self) -> Option<i64> {
        Some(i64::from_be_bytes(self.next_n::<8>()?))
    }

    fn next_f32(&mut self) -> Option<f32> {
        Some(f32::from_be_bytes(self.next_n::<4>()?))
    }

    fn next_f64(&mut self) -> Option<f64> {
        Some(f64::from_be_bytes(self.next_n::<8>()?))
    }

    fn next_string(&mut self, len: usize) -> Option<String> {
        match String::from_utf8(self.next_n_vec(len as usize)?) {
            Ok(str) => Some(str),
            Err(_) => None
        }
    }

    fn next_n_i8_vec(&mut self, n: usize) -> Option<Vec<i8>> {
        let mut bytes = Vec::new();
        for _ in 0..n {
            bytes.push(self.next_i8()?);
        }
        Some(bytes)
    }

    fn next_n_i32_vec(&mut self, n: usize) -> Option<Vec<i32>> {
        let mut ints = Vec::new();
        for _ in 0..n {
            ints.push(self.next_i32()?);
        }
        Some(ints)
    }

    fn next_n_i64_vec(&mut self, n: usize) -> Option<Vec<i64>> {
        let mut longs = Vec::new();
        for _ in 0..n {
            longs.push(self.next_i64()?);
        }
        Some(longs)
    }
}

struct Tag {
    name: String,
    payload: TagPayload,
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.name.len() == 0 {
            write!(f, "{}", self.payload)
        } else {
            write!(f, "\"{}\": {}", self.name, self.payload)
        }
    }
}

trait DumpContent {
    fn dump_content(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<T: fmt::Display> DumpContent for Vec<T> {
    fn dump_content(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.len() != 0 {
            write!(f, "{}", self[0])?;
            for i in 1..self.len() {
                write!(f, ", {}", self[i])?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for TagPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagPayload::Byte(x) => write!(f, "{}", x),
            TagPayload::Short(x) => write!(f, "{}", x),
            TagPayload::Int(x) => write!(f, "{}", x),
            TagPayload::Long(x) => write!(f, "{}", x),
            TagPayload::Float(x) => write!(f, "{}", x),
            TagPayload::Double(x) => write!(f, "{}", x),
            TagPayload::ByteArray(x) => write!(f, "{:?}", x),
            TagPayload::String(x) => write!(f, "\"{}\"", x),
            TagPayload::List(x) => {
                write!(f, "[ ")?;
                x.dump_content(f)?;
                write!(f, " ]")
            },
            TagPayload::Compound(x) => {
                write!(f, "{{ ")?;
                x.dump_content(f)?;
                write!(f, " }}")
            },
            TagPayload::IntArray(x) => write!(f, "{:?}", x),
            TagPayload::LongArray(x) => write!(f, "{:?}", x),
        }
    }
}

impl Tag {
    
    fn parse(iterator: &mut Iter<'_, u8>) -> Option<Tag> {

        let tag_id = iterator.next_u8()?;
        
        if tag_id == 0 {
            return None
        } else {
            
            let name_length = iterator.next_u16()?;

            let name = iterator.next_string(name_length as usize)?;

            Some(Tag {
                name,
                payload: Tag::parse_payload(iterator, tag_id)?,
            })
        }
    }

    fn parse_payload(iterator: &mut Iter<'_, u8>, tag_id: u8) -> Option<TagPayload> {

        match tag_id {
            1 => Some(TagPayload::Byte(iterator.next_i8()?)),
            2 => Some(TagPayload::Short(iterator.next_i16()?)),
            3 => Some(TagPayload::Int(iterator.next_i32()?)),
            4 => Some(TagPayload::Long(iterator.next_i64()?)),
            5 => Some(TagPayload::Float(iterator.next_f32()?)),
            6 => Some(TagPayload::Double(iterator.next_f64()?)),
            7 => {
                let arr_len = iterator.next_i32()? as usize;
                Some(TagPayload::ByteArray(iterator.next_n_i8_vec(arr_len)?))
            },
            8 => {
                let str_len = iterator.next_u16()? as usize;
                Some(TagPayload::String(iterator.next_string(str_len)?))
            },
            9 => {
                let tag_id = iterator.next_u8()?;
                let tags_count = iterator.next_i32()? as usize;
                let mut tag_list = Vec::new();
                
                for _ in 0..tags_count {
                    tag_list.push(Tag::parse_payload(iterator, tag_id)?);
                }

                Some(TagPayload::List(tag_list))
            },
            10 => {
                let mut tag_id = iterator.next_u8()?;
                let mut tag_list = Vec::new();

                while tag_id != 0 {
                    
                    let name_length = iterator.next_u16()?;
                    let name = iterator.next_string(name_length as usize)?;

                    tag_list.push(Tag{
                        name,
                        payload: Tag::parse_payload(iterator, tag_id)?,
                    });

                    tag_id = iterator.next_u8()?;
                }

                Some(TagPayload::Compound(tag_list))
            },
            11 => {
                let arr_len = iterator.next_i32()? as usize;
                Some(TagPayload::IntArray(iterator.next_n_i32_vec(arr_len)?))
            },
            12 => {
                let arr_len = iterator.next_i32()? as usize;
                Some(TagPayload::LongArray(iterator.next_n_i64_vec(arr_len)?))
            },
            _ => None,
        }
    }
}


enum TagPayload {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<TagPayload>),
    Compound(Vec<Tag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

trait GetPayloadByName {
    fn get_by_name(&mut self, name: &str) -> &mut TagPayload;
}

impl GetPayloadByName for Vec<Tag> {
    fn get_by_name(&mut self, name: &str) -> &mut TagPayload {
        for item in self {
            if item.name == name {
                return &mut item.payload;
            }
        }
        panic!("NBT format error");
    }
}

impl TagPayload {
    fn as_byte(&mut self) -> &mut i8 {
        if let TagPayload::Byte(b) = self {
            b
        } else {
            panic!("NBT format error");
        }
    }

    fn as_short(&mut self) -> &mut i16 {
        if let TagPayload::Short(s) = self {
            s
        } else {
            panic!("NBT format error");
        }
    }

    fn as_int(&mut self) -> &mut i32 {
        if let TagPayload::Int(i) = self {
            i
        } else {
            panic!("NBT format error");
        }
    }

    fn as_long(&mut self) -> &mut i64 {
        if let TagPayload::Long(l) = self {
            l
        } else {
            panic!("NBT format error");
        }
    }

    fn as_float(&mut self) -> &mut f32 {
        if let TagPayload::Float(f) = self {
            f
        } else {
            panic!("NBT format error");
        }
    }

    fn as_double(&mut self) -> &mut f64 {
        if let TagPayload::Double(d) = self {
            d
        } else {
            panic!("NBT format error");
        }
    }

    fn as_byte_array(&mut self) -> &mut Vec<i8> {
        if let TagPayload::ByteArray(ba) = self {
            ba
        } else {
            panic!("NBT format error");
        }
    }

    fn as_string(&mut self) -> &mut String {
        if let TagPayload::String(s) = self {
            s
        } else {
            panic!("NBT format error");
        }
    }

    fn as_list(&mut self) -> &mut Vec<TagPayload> {
        if let TagPayload::List(list) = self {
            list
        } else {
            panic!("NBT format error");
        }
    }
    
    fn as_compound(&mut self) -> &mut Vec<Tag> {
        if let TagPayload::Compound(comp) = self {
            comp
        } else {
            panic!("NBT format error");
        }
    }
    
    fn as_int_array(&mut self) -> &mut Vec<i32> {
        if let TagPayload::IntArray(ia) = self {
            ia
        } else {
            panic!("NBT format error");
        }
    }
    
    fn as_long_array(&mut self) -> &mut Vec<i64> {
        if let TagPayload::LongArray(la) = self {
            la
        } else {
            panic!("NBT format error");
        }
    }

}

fn parse_chunks(f: &mut File, chunk_offsets: &Vec<u64>) -> Result<Vec<Tag>> {
    let mut chunks = Vec::new();
    let mut buf4: [u8; 4] = [0; 4]; 

    for (i, chunk_offset) in chunk_offsets.iter().enumerate() {
        f.seek(SeekFrom::Start(*chunk_offset))?;

        f.read_exact(&mut buf4)?;
        let chunk_length = u32::from_be_bytes(buf4);
        println!("Chunk {i} has length: {chunk_length}");

        let mut buf1: [u8; 1] = [0; 1]; 

        f.read_exact(&mut buf1)?;

        ensure!(buf1[0] == 2); // Compression type gzip

        let mut chunk_data = vec![0u8; chunk_length as usize];
        f.read_exact(&mut chunk_data)?;

        let mut decompressed: Vec<u8> = Vec::new();

        ZlibDecoder::new(chunk_data.as_slice()).read_to_end(&mut decompressed)?;

        
        let mut iterator = decompressed.iter();

        if let Some(root) = Tag::parse(&mut iterator) {
            chunks.push(root);
        } else {
            eprintln!("Could not parse chunk {i} :(");
        }

    }

    println!("{}/{} chunks parsed successfully", chunks.len(), chunk_offsets.len());

    return Ok(chunks);
}

struct BlockType {
    name: String
}

enum Block {
    Skip(usize),
    Type(usize)
}

fn main() -> Result<()> {
    // let (mut rl, thread) = raylib::init()
    //     .size(640, 480)
    //     .title("Hello, World")
    //     .build();
     
    // while !rl.window_should_close() {
    //     let mut d = rl.begin_drawing(&thread);
         
    //     d.clear_background(Color::WHITE);
    //     d.draw_text("Hello, world!", 12, 12, 20, Color::BLACK);
    // }

    let mut f = File::open("resources/r.0.0.mca")?;

    let mut buf4: [u8; 4] = [0; 4]; 

    let mut chunk_offsets: Vec<u64> = Vec::new();

    for _ in 0..1024 {
        f.read_exact(&mut buf4)?;
        let chunk_loc = chunk_loc_to_byte_offset(buf4);
        if let Some(chunk_loc) = chunk_loc {
            chunk_offsets.push(chunk_loc);
        } else {
            break;
        }
    }

    println!("Chunks: {}", chunk_offsets.len());

    println!("Chunk offset 0: {}", chunk_offsets[0]);

    let mut chunks = parse_chunks(&mut f, &chunk_offsets)?;

    // println!("{}", chunks[0]);

    let block_type_index: HashMap<String, usize> = HashMap::new();
    let block_type: Vec<BlockType> = Vec::new();

    let sections = chunks[0].payload.as_compound().get_by_name("sections").as_list();

    for section in sections {
        println!("\nNew palette:");
        let palette = section.as_compound().get_by_name("block_states").as_compound().get_by_name("palette").as_list();

        for block in palette {
            println!("Found: {}", block.as_compound().get_by_name("Name").as_string());
        }
    }
    

    // println!("Chunk 0 section count: {}", sections.len());

    Ok(())
}
