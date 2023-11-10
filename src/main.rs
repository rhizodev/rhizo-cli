mod client;
mod utils;
mod commands;
mod error;
mod types;

use crate::commands::*;

fn print_help(){
    println!("rhizo-cli\n");
    println!("Commands");
    println!("deploy $wasm_module_path $route_config_path\tDeploy a route configuration and backing WASM module. Both should validate locally.");
    println!("help\t\t\t\t\t\tView the help information for this tool.");
    println!("ls [socb | route]\t\t\t\tFetch the list of the developer's hosted routes or signed onchain bytes.");   
    println!("socb alloc $key $num_bytes\t\t\tAllocate signed on-chain bytes owned by the current keypair.");
    println!("socb write $key $content_path\t\t\tWrite signed on-chain bytes owned by the current keypair.");
    println!("validate-config $route_config_path\t\tValidate a route configuration file");   
    println!("validate-module $wasm_module_path\t\tValidate a compiled WASIX WASM file. Only detects ABI compatibility with rhizo-server, not runtime errors.");        
    println!("version\t\t\t\t\t\tView the rhizo-cli version.");
    println!("view [socb | route] $key\t\t\tFetch a route or signed onchain bytes by name.");
    println!("test-module $wasm_module_path\t\t\tExecute the module's test() function locally. Useful for local testing and finding runtime errors.");
    println!("yank $route_key\t\t\t\t\tInitiate yanking a route from the network.")
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), ()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1) {
        Some(command) => {
            if command.to_lowercase().eq("deploy"){
                let try_wasm_path = args.get(2);
                let try_api_config_path = args.get(3);
                match (try_wasm_path, try_api_config_path) {
                    (Some(wasm_path), Some(config_path)) => {
                        match deploy(wasm_path, config_path, 0u8).await {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("[ERROR] deploy failed due to error: {}", err.message)
                            }
                        }
                    }
                    _ => {}
                }
            } else
            if command.to_lowercase().eq("yank"){
                let try_seed = args.get(2);
                match try_seed {
                    Some(seed) => {
                        match yank_route(seed) {
                            Ok(_) => { }
                            Err(err) => {
                                eprintln!("[ERROR] yank failed due to error: {}", err.message)
                            }
                        }
                    }
                    _ => { println!("Missing arguments") }
                }
            }
            else if command.to_lowercase().eq("view"){
                let collection = args.get(2).unwrap();
                let try_seed = args.get(3);
                match try_seed {
                    Some(seed) => {
                        match view(collection, seed) {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("[ERROR] view failed due to error: {}", err.message)
                            }
                        }
                    }
                    _ => {}
                }
            }
            else if command.to_lowercase().eq("ls"){
                    let collection = args.get(2).unwrap();
                        match ls(collection) {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("[ERROR] ls failed due to error: {}", err.message)
                            }
                        }
            }
            else if command.to_lowercase().eq("validate-config"){
                let try_cid = args.get(2);
                match try_cid {
                    Some(cid) => {
                        match validate_config(cid).await {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("[ERROR] view failed due to error: {}", err.message)
                            }
                        }
                    }
                    _ => {}
                }
            } else if command.to_lowercase().eq("validate-module"){
                let try_cid = args.get(2);
                match try_cid {
                    Some(cid) => {
                        match validate_module(cid).await {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("[ERROR] view failed due to error: {}", err.message)
                            }
                        }
                    }
                    _ => {}
                }
            } else if command.to_lowercase().eq("socb") { 
                let ocb_cmd = args.get(2).unwrap();
                let ocb_seed = args.get(3).unwrap();
                let ocb_content = args.get(4).unwrap();
               
                if ocb_cmd.eq("write"){
                    match ocb_write(ocb_seed, ocb_content.as_bytes().to_vec()) {
                        Err(e) => println!("[ERROR] socb write failed due to: {}", e),
                        Ok(_) => {}
                    } 
                } else if ocb_cmd.eq("alloc") {
                    match ocb_alloc(ocb_seed, 32) {
                        Err(e) => println!("[ERROR] socb alloc failed due to: {}", e),
                        Ok(_) => {}
                    }
                }
            } else if command.to_lowercase().eq("test-module") {
                test_module(args.get(2).unwrap());
            } else if command.to_lowercase().eq("-h") || command.to_lowercase().eq("--help") || command.to_lowercase().eq("help") { 
                print_help();
            } else if command.to_lowercase().eq("-v") || command.to_lowercase().eq("--version") || command.to_lowercase().eq("version") { 
                println!("rhizo-cli {VERSION}");
            } else {
                println!("[ERROR] unsupported command {:?}", command);
                print_help();
            }
        }
        _ => {
            println!("[ERROR] missing command");
            print_help();
        }
    } 
    Ok(())
}
