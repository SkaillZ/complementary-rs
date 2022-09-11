mod objects;

use std::{
    env,
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    io::{BufReader, Cursor, Seek},
    iter,
    path::Path,
};

use binrw::{until_eof, BinRead, FilePtr64};
use objects::FVec2;
use serde::Serialize;
use walkdir::WalkDir;

use crate::objects::convert_object_data;

enum FileType {
    ObjectMap, // CMOM files
    Object,    // CMOB files
}

pub fn main() {
    let orig_path = env::args()
        .nth(1)
        .expect("Pass the path to the original assets as the first argument");
    let target_path = fs::canonicalize("assets/").expect("Assets directory missing");
    for entry in WalkDir::new(&orig_path) {
        let entry = entry.unwrap();
        let ext = entry.path().extension();

        let file_type = if ext == Some(OsStr::new("cmom")) {
            FileType::ObjectMap
        } else if ext == Some(OsStr::new("cmob")) {
            FileType::Object
        } else {
            continue;
        };

        let relative_path = entry.path().strip_prefix(&orig_path).unwrap();

        let mut target_file_path = target_path.join(relative_path);
        target_file_path.set_extension("json");

        if let Some(parent) = target_file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create directory");
        }

        let result = match file_type {
            FileType::Object => convert_single_object_file(entry.path(), &target_file_path),
            FileType::ObjectMap => convert_object_map(entry.path(), &target_file_path),
        };
        if let Err(error) = result {
            eprintln!("Failed to convert '{}': {}", relative_path.display(), error);
        }
    }
}

#[derive(Debug, BinRead)]
#[br(little, magic = b"CMOB")]
struct ObjectBin {
    prototype_id: i32,
    position: FVec2,
    #[br(parse_with = until_eof)]
    data: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct ObjectJson {
    r#type: &'static str,
    position: FVec2,
    data: serde_json::Value,
}

pub fn convert_single_object_file(
    source_path: &Path,
    target_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = BufReader::new(File::open(source_path)?);
    let object = ObjectBin::read(&mut file)?;

    let mut data = object.data;
    if data.len() < 128 {
        // Some assets weren't rebuilt with the latest version, so zeroes are missing at the end
        // This works in the C++ version since the data is `memcpy`'d into an empty struct of the correct size
        data.extend(iter::repeat(0).take(128 - data.len()));
    }
    assert_eq!(data.len(), 128);

    let mut data = Cursor::new(data);
    let (r#type, json_data) = convert_object_data(object.prototype_id, &mut data)?;

    let json_contents = ObjectJson {
        r#type,
        position: object.position,
        data: json_data,
    };
    let json_str = serde_json::to_string_pretty(&json_contents)?;
    fs::write(target_path, json_str)?;

    Ok(())
}

#[derive(Debug, BinRead)]
#[br(little, magic = b"CMOM")]
struct ObjectMapBin {
    start_pointer: FilePtr64<ObjectMapBinMain>,
}

#[derive(Debug, BinRead)]
#[br(little)]
struct ObjectMapBinMain {
    _object_num: i32,
    #[br(count = _object_num)]
    objects: Vec<ObjectMapBinItem>,
}

#[derive(Debug, BinRead)]
struct ObjectMapBinItem {
    prototype_id: i32,
    position: FVec2,
    data_offset: i32,
}

pub fn convert_object_map(source_path: &Path, target_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut file = BufReader::new(File::open(source_path)?);
    let object_map = ObjectMapBin::read(&mut file)?.start_pointer;

    let objs: Vec<ObjectJson> = object_map
        .objects
        .iter()
        .filter_map(|object| {
            file.seek(std::io::SeekFrom::Start(object.data_offset as u64))
                .ok();
            let (r#type, json_data) = convert_object_data(object.prototype_id, &mut file)
                .map_err(|err| {
                    eprintln!("Error while converting {}: {}", source_path.display(), err);
                    err
                })
                .ok()?;

            Some(ObjectJson {
                r#type,
                position: object.position,
                data: json_data,
            })
        })
        .collect();

    let json_str = serde_json::to_string_pretty(&objs)?;
    fs::write(target_path, json_str)?;

    Ok(())
}
