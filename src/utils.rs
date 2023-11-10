use crate::error::RhizoCLIError as Error;
use solana_sdk::signer::keypair::{Keypair, read_keypair_file};
use rhizo_types::{ArgumentType::*, ArgumentType};
use rhizo_types::CollectionType;
use rhizo_types::NestedCollectionType;
use yaml_rust::YamlLoader;
use yaml_rust::Yaml;

pub fn get_rpc_url() -> Result<String, Error> {
    get_config().and_then(|maybe_yaml| {
        maybe_yaml
            .ok_or("YAML config exists but may be empty.")
            .and_then(|config| {
                config["json_rpc_url"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or("json_rpc_url could not be parsed as str")
            })
            .map_err(|e| Error::new(e))
    })
}

pub fn solana_config_path() -> Result<std::path::PathBuf, Error> {
    if let Some(mut path) = home::home_dir() {
        path.push(".config/solana/cli/config.yml");
        return Ok(path) 
    } 
    Err(Error::new("Unable to find home dir"))
}

pub fn get_config() -> Result<Option<Yaml>, Error> {
    solana_config_path()
        .and_then(|path| {
            std::fs::read_to_string(path)
                .map_err(|_| Error::new("Unable to read contents at Solana config path"))
        })
        .and_then(|contents| {
            YamlLoader::load_from_str(&contents)
                .map_err(|_| Error::new("Unable to parse contents at Solana config path to yaml"))
        })
        .and_then(|configs| Ok(configs.last().map(|c| c.to_owned())))
}

pub fn get_keypair() -> Result<Keypair, Error> {
    get_config().and_then(|maybe_yaml| {
            maybe_yaml.ok_or("YAML config exists but may be empty.")
            .map_err(|e| Error::new(e))
            .and_then(|config| {
                config["keypair_path"]
                    .as_str()
                    .ok_or("Could not parse keypair_path as str")
                    .map_err(|e| Error::new(e)) 
                    .and_then(|s| {
                        read_keypair_file(s.to_string())
                            .map_err(|_| Error::new("Could not parse file pointed to by keypath_pair as a Solana Keypair"))
                    }) 
            })
            .map_err(|e| Error::new(e.to_string().as_str()))
    })
}

pub fn parse_argument_type(argument_type: String) -> Result<ArgumentType, Error> {
    match argument_type.to_lowercase().as_str() {
            "u8" => { return Ok(U8) }
            "u16" => { return Ok(U16) }
            "u32" => { return Ok(U32) }
            "u64" => { return Ok(U64) }
            "i8" => { return Ok(I8) }
            "i16" => { return Ok(I16)}
            "i32" => { return Ok(I32)}
            "i64" => { return Ok(I64)}
            "f32" => { return Ok(F32)}
            "f64" => { return Ok(F64) }           
            "str" => { return Ok(Str) }
            "bool" => { return Ok(Bool) }
            "vec<u8>" => { return Ok(Array(CollectionType::U8))  }
            "vec<u16>" => { return Ok(Array(CollectionType::U16)) }
            "vec<u32>" => { return Ok(Array(CollectionType::U32)) }
            "vec<u64>" => { return Ok(Array(CollectionType::U64)) }
            "vec<i8>" => { return Ok(Array(CollectionType::I8)) }
            "vec<i16>" => { return Ok(Array(CollectionType::I16)) }
            "vec<i32>" => { return Ok(Array(CollectionType::I32)) }
            "vec<i64>" => { return Ok(Array(CollectionType::I64)) }
            "vec<f32>" => { return Ok(Array(CollectionType::F32)) }
            "vec<f64>" => { return Ok(Array(CollectionType::F64)) }
            "vec<str>" => { return Ok(Array(CollectionType::Str)) }
            "vec<bool>" => { return Ok(Array(CollectionType::Bool)) }            
            "vec<vec<u8>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::U8)))  }
            "vec<vec<u16>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::U16))) }
            "vec<vec<u32>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::U32))) }
            "vec<vec<u64>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::U64))) }
            "vec<vec<i8>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::I8))) }
            "vec<vec<i16>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::I16))) }
            "vec<vec<i32>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::I32))) }
            "vec<vec<i64>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::I64))) }
            "vec<vec<f32>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::F32))) }
            "vec<vec<f64>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::F64))) }
            "vec<vec<str>>" => { return Ok(Array(CollectionType::Array(NestedCollectionType::Str)))  }  
            other => {
                return Err(Error::new(format!("Unsupported argument type {}", other).as_str()))
            }
        }
}
