use borsh::{BorshSerialize, BorshDeserialize};
use crate::{client, error::RhizoCLIError as Error, utils};
use crate::types::Config;
use hyper::{Body, Client, Request, StatusCode};
use hyper::header::CONTENT_TYPE;
use rhizo_types::Encoding::*;
use solana_sdk::{pubkey::Pubkey, signature::Signer};
use solana_program::pubkey::Pubkey as ProgramPubkey;
use std::io::Write;
use std::str::FromStr;
use spinners::{Spinner, Spinners};
use wasmer::{Module, Store};
use wasmer_wasix::Pipe;
use wasmer_wasix::{WasiEnvBuilder, capabilities::Capabilities, http::HttpClientCapabilityV1, capabilities::CapabilityThreadingV1};

pub fn yank_route(seed: &str) -> Result<(), Error> {
    let keypair = utils::get_keypair()?;
    let program_pubkey = Pubkey::from_str("Ep1SV45cqumZmogwWFy6pVNvMpRerMZUUhSJTbTh2e58");

    if program_pubkey.is_err() {        
        return Err(Error::new("Unable to create pubkey for pre-configured program address"))
    }

    let connection = client::establish_connection()?;

    client::yank_route(&keypair, &program_pubkey.unwrap(), &connection, seed).unwrap();
    Ok(())
}

pub fn test_module(path: &str){
    println!("WARNING: Failing to use rhizo_sdk functions like tprintln and assert_eq as the exclusive methods to print output and assert within your test function may result in hanging tests.");
    let mut store = Store::default();
    let mut module = Module::from_file(&store, path).expect("Failed to read Wasm module");

    let (mut stdin_tx, stdin_rx) = Pipe::channel();    
    let (stderr_tx, mut stderr_rx) = Pipe::channel();
    let (stdout_tx, mut stdout_rx) = Pipe::channel();

    let argument_buffer = Vec::<u8>::new();
    let len_bytes = (argument_buffer.len() as i32).to_le_bytes();
    let concatenated = [len_bytes.to_vec(), argument_buffer].concat();
    stdin_tx.write_all(concatenated.as_slice()).unwrap();
    stdin_tx.flush().unwrap();

    let wasi_env_builder = WasiEnvBuilder::new("wasm")
        .capabilities(
        Capabilities {
            insecure_allow_all: true,
            http_client: HttpClientCapabilityV1::new_allow_all(),
            threading: CapabilityThreadingV1 { max_threads: None, enable_asynchronous_threading: true },
    });
    let (mut instance, mut fn_env) = wasi_env_builder.instantiate(module.clone(), &mut store).unwrap();        
    let try_get_start_fn = instance.exports.get_function("test");
    if try_get_start_fn.is_err() {
        println!("test() function not found in the provided module. Check that #[no_mangle] was defined above the function implementation.");
        return
    }
    let start_fn = try_get_start_fn.unwrap();
    
    let invoke_res = start_fn.call(&mut store, &[]); 
    
    if invoke_res.is_err(){
        println!("Failed module tests.");
        return
    }
        
    println!("All assertions passed.");
}

pub fn view(collection: &str, seed: &str) -> Result<(), Error> {
    let mut spinner = Spinner::new(Spinners::Dots8Bit, format!("Fetching {} config..", collection).into());
    let connection = client::establish_connection()?;
    let keypair = utils::get_keypair()?;

    let program_pubkey = match Pubkey::from_str(
        "Ep1SV45cqumZmogwWFy6pVNvMpRerMZUUhSJTbTh2e58"
    ) {
        Ok(pubkey) => pubkey,
        _ => {
            return Err(Error::new("Unable to create a pubkey for the program address"));
        }
    };

    let seed = format!("{collection}-{seed}");

    let (_, bump_seed) = ProgramPubkey::find_program_address(
        &[seed.as_bytes(),
          keypair.pubkey().to_bytes().as_slice()],
          &program_pubkey    
    );

    let pda_address = ProgramPubkey::create_program_address(&[seed.as_bytes(), keypair.pubkey().to_bytes().as_slice(), &[bump_seed]], &program_pubkey);

    if pda_address.is_err() {
        return Err(Error::new("Unable to create a PDA the view command"));
    }

    let account_data = connection.get_account_data(&pda_address.clone().unwrap());
    if account_data.is_err(){
        return Err(Error::new("Unable to fetch account data"))
    }
    let account_data = account_data.unwrap();

    let mut buffer = account_data.as_slice();
    

    spinner.stop_with_symbol("🗸");
    if collection.eq("route"){
    let deserialized = match rhizo_types::RouteData::deserialize(&mut buffer) {
        Ok(route_data) => route_data, 
        _ => return Err(Error::new("Unable to deserialize account data as RouteData")),
    };
    let hash = iroh_blake3::Hash::from_bytes(deserialized.module_cid);
    println!("Route:\t\t\t{:?}", deserialized.route);    
    println!("Module CID:\t\t{:?}", hash.to_string());
    println!("Supported Encodings:\t{:?}", deserialized.encodings);    
    println!("Arguments:");
    deserialized.arguments.into_iter().for_each(|arg| println!("\t\t\t{}: {:?}", std::str::from_utf8(arg.0.as_slice()).unwrap(), arg.1));
    }
    if collection.eq("socb"){
    let deserialized = match rhizo_types::SignedOnchainBytes::deserialize(&mut buffer) {
        Ok(route_data) => route_data, 
        _ => return Err(Error::new("Unable to deserialize account data as RouteData")),
    };
    println!("PDA: \t\t\t{:?}", pda_address.unwrap());
    println!("Owner Pubkey: \t\t{:?}", Pubkey::new_from_array(deserialized.owner_pubkey));
    println!("Contents: \t\t{:?}", deserialized.inner);
    }
    Ok(())
}

pub fn ls(collection: &str) -> Result<(), Error> {
    let mut spinner = Spinner::new(Spinners::Dots8Bit, format!("Fetching hosted {}s..", collection).into());
    let connection = client::establish_connection()?;
    let keypair = utils::get_keypair()?;

    let program_pubkey = match Pubkey::from_str(
        "Ep1SV45cqumZmogwWFy6pVNvMpRerMZUUhSJTbTh2e58"
    ) {
        Ok(pubkey) => pubkey,
        _ => {
            return Err(Error::new("Unable to create a pubkey for the program address"));
        }
    };

    let seed = {
     if collection == "socb" {
        "_dev_socbs"
     } else if collection == "route" {
        "_dev_routes"
     } else {
        panic!("invalid collection, could not list")
     }
    };

    let (_, bump_seed) = ProgramPubkey::find_program_address(
        &[seed.as_bytes(),
          keypair.pubkey().to_bytes().as_slice()],
          &program_pubkey    
    );

    let pda_address = ProgramPubkey::create_program_address(&[seed.as_bytes(), keypair.pubkey().to_bytes().as_slice(), &[bump_seed]], &program_pubkey);

    if pda_address.is_err() {
        return Err(Error::new("Unable to create a PDA the view command"));
    }

    let account_data = connection.get_account_data(&pda_address.unwrap());
    if account_data.is_err(){
        return Err(Error::new("Unable to fetch account data"))
    }
    let account_data = account_data.unwrap();

    let mut buffer = account_data.as_slice();
    let deserialized = match rhizo_types::DeveloperRoutes::deserialize(&mut buffer) {
        Ok(route_data) => route_data, 
        _ => return Err(Error::new("Unable to deserialize account data as RouteData")),
    };

    spinner.stop_with_symbol("🗸");

    for r in deserialized.routes {
        println!("{}", r);
    }

    Ok(())
}

pub async fn validate_config(config_path: &str) -> Result<(), Error> {
    let toml_str = std::fs::read_to_string(config_path);
    if toml_str.is_err(){
        return Err(Error::new("Unable to read file at the provided route configuration path as a String"))
    }

    let config: Result<Config, _> = toml::from_str(&toml_str.unwrap());
    if config.is_err(){
        return Err(Error::new("Unable to read file at provided path as a TOML file"))
    }
    
    let config = config.unwrap();

    println!("Route:\t\t\t{:?}", config.route);
    println!("Supported Encodings:\t{:?}", config.encodings);    
    println!("Arguments:");
    config.arguments
        .into_iter()
        .for_each(
            |arg| println!("\t\t\t{}: {}", std::str::from_utf8(arg.name.as_bytes()).unwrap(), arg.argument_type)
        );

    Ok(())
}

pub async fn validate_module(wasm_path: &str) -> Result<(), Error> {
    let mut store = Store::default();
    let module = Module::from_file(&store, wasm_path);
    if module.is_err(){
        return Err(Error::new("Unable to read file at provided path as a valid WASM module"));
    }
    let (mut stdin_tx, stdin_rx) = Pipe::channel();    
    let (stderr_tx, _) = Pipe::channel();

    let arg_bytes = Vec::<u8>::new();
    let len_bytes = (arg_bytes.len()as i32).to_le_bytes();
    let concatenated = [len_bytes.to_vec(), arg_bytes].concat();
    stdin_tx.write_all(concatenated.as_slice()).unwrap();
    stdin_tx.flush().unwrap();

    let wasi_env_builder = WasiEnvBuilder::new("wasm")
    .capabilities(
            Capabilities {
                insecure_allow_all: true,
                http_client: HttpClientCapabilityV1::new_allow_all(),
                threading: CapabilityThreadingV1 { max_threads: None, enable_asynchronous_threading: true },
    })
    .stderr(Box::new(stderr_tx))
    .stdin(Box::new(stdin_rx))
    .stdout(Box::new(wasmer_wasix::virtual_fs::host_fs::Stdout::default()));

    let instance = Box::new(wasi_env_builder.instantiate(module.unwrap(), &mut store).unwrap().0);
    let get_start = instance.exports.get_function("_start");
    if get_start.is_err() {
        return Err(Error::new("Unable to find _start function in the provided WASM module"))
    }
    println!("WASM file {:?} passed a compile-level module validation", wasm_path); 
    Ok(()) 
}

pub async fn deploy(wasm_path: &str, config_path: &str, operation_byte: u8) -> Result<(), Error> {
    let connection = client::establish_connection().map_err(|e| { 
        println!("RPC client establish connection failed {:?}", e);
        Error::new("Unable to establish RPC connections")
    })?;
    let keypair = utils::get_keypair()?;
    let program_pubkey = Pubkey::from_str("Ep1SV45cqumZmogwWFy6pVNvMpRerMZUUhSJTbTh2e58");

    if program_pubkey.is_err() {        
        return Err(Error::new("Unable to create pubkey for pre-configured program address"))
    }

    let toml_str = std::fs::read_to_string(config_path);
    if toml_str.is_err(){
        return Err(Error::new("Unable to find a file at the provided route configuration path"))
    }

    let config: Result<Config, _> = toml::from_str(&toml_str.unwrap());
    if config.is_err(){
        return Err(Error::new("Unable to parse route config as TOML"))
    }
    let config = config.unwrap();
    let config = Config {route: format!("route-{}", config.route), encodings: config.encodings, arguments: config.arguments, cacheable: config.cacheable, cache_ttl_ms: config.cache_ttl_ms};

    let mut encodings: Vec<rhizo_types::Encoding> = vec![];
    let mut arguments: Vec<(Vec<u8>, rhizo_types::ArgumentType)> = vec![];

    for encoding in config.encodings {
        match encoding.to_lowercase().as_str() {
            "texthtml" => { encodings.push(TextHtml); }
            "textplain" => { encodings.push(TextPlain) }
            "applicationoctetstream" => { encodings.push(ApplicationOctetStream) }
            "applicationjson" => { encodings.push(ApplicationJson) }
            _ => {}
        }
    }

    for argument in config.arguments {
            arguments.push((argument.name.into_bytes(), utils::parse_argument_type(argument.argument_type)?));
    }

    let wasm_source = std::fs::read(wasm_path);
    if wasm_source.is_err(){
        return Err(Error::new("Unable to find content at the WASM source path provided"))
    }
    let wasm_source = wasm_source.unwrap();

    let mut hasher = iroh_blake3::Hasher::new();
    hasher.update(wasm_source.as_slice());
    let hash = hasher.finalize();

    let mut cid_bytes: [u8; 32] = [0u8; 32];
  
    cid_bytes.copy_from_slice(hash.as_bytes());

    let route_data = rhizo_types::RouteData {
        route: config.route,
        module_cid : cid_bytes,
        encodings: encodings.clone(),
        arguments: arguments.clone(),
        bump_seed: None,
        cache_config: (config.cacheable, config.cache_ttl_ms)
    };
    
    let route_source = rhizo_types::RouteDeploy {
        metadata: route_data.clone(),
        source: wasm_source,
    };
    
    let _ = client::update_route_data(&keypair, &program_pubkey.unwrap(), &connection, &route_data, operation_byte)?;

    if operation_byte.eq(&1u8) {
        return Ok(())
    }

    let mut spinner = Spinner::new(Spinners::Dots8Bit, "Pushing module & route config to devnet".into());

    let client = Client::new();
    let request = Request::builder()
        .method("POST")
        .uri("http://euro.rhizo.dev/ingest")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(route_source.try_to_vec().expect("route data serializes")))
        .unwrap();

    let response = client.request(request).await.unwrap();
   
    if response.status().eq(&StatusCode::OK){
        let body_bytes = hyper::body::to_bytes(response.into_body()).await;
        if body_bytes.is_err(){
            return Err(Error::new("Unable to read response bytes from Rhizo server"))
        }
        let body_bytes = body_bytes.unwrap();
        let body_str = std::str::from_utf8(&body_bytes);
        if body_str.is_err(){
            return Err(Error::new("Unable to parse respoonse body as UTF-8"))
        }
        spinner.stop_with_symbol("🗸");
        println!("-------------------------------");        
        println!("Route:\t\t\t{:?}", route_data.route);
        println!("Module CID:\t\t{:?}", hash.to_string());
        println!("Supported Encodings:\t{:?}", route_data.encodings);    
        println!("Arguments:");
        route_data.arguments
            .into_iter()
            .for_each(
                |arg| println!("\t\t\t{}: {:?}", std::str::from_utf8(arg.0.as_slice()).unwrap(), arg.1)
            );
    } else {
        if response.status().as_str().eq("413"){
            return Err(Error::new("Payload too large. WASM module must gzip to less than 2mb."))
        } else {
            return Err(Error::new(&format!("Error from rhizo server {:?}", response.status().as_str())))
        }
        //return Err(Error::new(format!("Error from Rhizo server {:?}", response.status()).as_str()))
    }
    Ok(())
}

pub fn ocb_alloc(seed: &str, size: usize) -> Result<(), Error> {
    let keypair = utils::get_keypair()?;
    let program_pubkey = Pubkey::from_str("Ep1SV45cqumZmogwWFy6pVNvMpRerMZUUhSJTbTh2e58")
        .map_err(|_| Error::new("Unable to create program pubkey"))?; 
    let connection = client::establish_connection().map_err(|_| Error::new("Unable to establish RPC connections"))?; 
    client::alloc_ocb(&keypair, &program_pubkey, &connection, &rhizo_types::SignedOnchainBytesUpdate { seed: seed.to_string(), bytes: rhizo_types::SignedOnchainBytes { inner: vec![0u8; size], owner_pubkey: keypair.pubkey().to_bytes()}, bump_seed: None, })    
}

pub fn ocb_write(seed: &str, content: Vec<u8>) -> Result<(), Error> {
    let keypair = utils::get_keypair()?;
    let program_pubkey = Pubkey::from_str("Ep1SV45cqumZmogwWFy6pVNvMpRerMZUUhSJTbTh2e58")
        .map_err(|_| Error::new("Unable to create program pubkey"))?;
   
    let connection = client::establish_connection().map_err(|_| Error::new("Unable to establish RPC connections"))?; 
    client::write_ocb(&keypair, &program_pubkey, &connection, &rhizo_types::SignedOnchainBytesUpdate { seed: seed.to_string() , bytes: rhizo_types::SignedOnchainBytes { inner: content, owner_pubkey: keypair.pubkey().to_bytes() }, bump_seed: None, })    
}
