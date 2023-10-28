use b64::ToBase64;
use borsh::BorshSerialize;
use crate::{utils, error::RhizoCLIError as Error};
use solana_client::rpc_client::RpcClient;
use solana_client::client_error::ClientError;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message;
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::transaction::Transaction;
use solana_sdk::transaction::TransactionError;
use solana_sdk::instruction::InstructionError;
use solana_sdk::pubkey::Pubkey;
use solana_program::system_program;
use solana_program::pubkey::Pubkey as ProgramPubkey;
use reqwest::header::CONTENT_TYPE;
use serde_json::{Map, Value};

pub fn establish_connection() -> Result<RpcClient, Error> {
    let rpc_url = utils::get_rpc_url()?;
    Ok(RpcClient::new_with_commitment(
        rpc_url,
        CommitmentConfig::confirmed(),
    ))
}

pub fn alloc_ocb(
    caller: &Keypair,
    program_pubkey: &Pubkey,
    connection: &RpcClient,
    update_data: &rhizo_types::SignedOnchainBytesUpdate,
) -> Result<(), Error> {
    let mut spinner = spinners::Spinner::new(spinners::Spinners::Dots8Bit, "Preparing Solana transaction..".into());
    let seed = &update_data.seed;
    let seed = format!("socb-{seed}");

    let (_, bump_seed) = ProgramPubkey::find_program_address(
        &[seed.as_bytes(),
          caller.pubkey().to_bytes().as_slice()],
          program_pubkey    
    );
    
    let created_pda = ProgramPubkey::create_program_address(&[seed.as_bytes(), caller.pubkey().to_bytes().as_slice(), &[bump_seed]], program_pubkey).unwrap();
  
    let update_request = rhizo_types::SignedOnchainBytesUpdate {
        seed: seed.to_owned(),
        bytes: update_data.bytes.to_owned(),
        bump_seed: Some(bump_seed),
    };

    println!("OCB PDA: {:?}", created_pda);

    let (_, bump_seed) = ProgramPubkey::find_program_address(
        &[b"_dev_socbs",
          caller.pubkey().to_bytes().as_slice()],
          program_pubkey    
    );
    
    let list_pda = ProgramPubkey::create_program_address(&[b"_dev_socbs", caller.pubkey().to_bytes().as_slice(), &[bump_seed]], program_pubkey).unwrap();

    let list_request = rhizo_types::ListSignedOnchainBytesUpdate {
        seed: seed.to_owned()
    };

    let mut list_request_vec = 5u64.to_le_bytes().to_vec();
    list_request_vec.extend(vec![bump_seed]);
    list_request_vec.extend(list_request.try_to_vec().expect("ocb can be listed"));

    let mut update_request_vec = 2u64.to_le_bytes().to_vec(); // Instruction marker for update ocb
    update_request_vec.extend(update_request.try_to_vec().expect("ocb update data can be serialized"));

    let request_update_instruction = Instruction {
        program_id: program_pubkey.to_owned(),
        accounts: vec![
            AccountMeta { pubkey: caller.pubkey(), is_signer: true, is_writable: true },
            AccountMeta { pubkey: created_pda, is_signer: false, is_writable: true },
            AccountMeta { pubkey: system_program::id(), is_signer: false, is_writable: false }
        ],
        data: update_request_vec
    };

    let list_socb_instruction = Instruction {
        program_id: program_pubkey.to_owned(),
        accounts: vec![
            AccountMeta { pubkey: caller.pubkey(), is_signer: true, is_writable: true },
            AccountMeta { pubkey: list_pda, is_signer: false, is_writable: true },
            AccountMeta { pubkey: system_program::id(), is_signer: false, is_writable: false }
        ],
        data: list_request_vec
    };

    let message = Message::new(&[
                               list_socb_instruction.clone(),
                               request_update_instruction.clone(), 
    ], Some(&caller.pubkey()));

    //if false{
    
    let fee_lamports = connection.get_fee_calculator_for_blockhash(&connection.get_latest_blockhash().unwrap()).unwrap().unwrap().calculate_fee(&message);
    let rent_exemption = connection.get_minimum_balance_for_rent_exemption(request_update_instruction.data.len()).unwrap();
    spinner.stop_with_symbol("🗸");        
    let confirmation = dialoguer::Confirm::new()
        .with_prompt(format!("  Deploy route operation costs an estimated minimum of {:?} lamports + an additional {:?} to initialize a route keyed by {:?}, continue?", fee_lamports,rent_exemption, seed.clone()))
        .interact()
        .unwrap();

    if !confirmation {
        return Err(Error::new("Unable to submit transaction - cancelled due to user input"))
    }
    //}
    let mut spinner = spinners::Spinner::new(spinners::Spinners::Dots8Bit, "Sending Solana transaction..".into());
    
    let transaction =
        Transaction::new(&[caller], message, connection.get_latest_blockhash().expect("can fetch recent blockhash"));
   
    println!("{:?}", bincode::serialize(&transaction).unwrap().to_base64(b64::URL_SAFE));


    let try_transaction = connection.send_and_confirm_transaction(&transaction);
    if try_transaction.is_err() {
        println!("TransactionError {:?}", try_transaction);
        let transaction_error = try_transaction.err().unwrap().get_transaction_error().unwrap();
        let error_message = match transaction_error {
            TransactionError::InstructionError(index, InstructionError::Custom(error_code)) => {
                match (index, error_code) {
                    (_, 0u32) => "Developer has reached the smart-contract's configured route limit.".to_string(),
                    (_, 1u32) => "Nice try.".to_string(),
                    _ => "Unsupported InstructionError code".to_string()
                }
            }
            other => { other.to_string() }
        };
        return Err(Error::new(error_message.as_str()))
    }
    
    spinner.stop_with_symbol("🗸");

    Ok(()) 
}

pub fn write_ocb(
    caller: &Keypair,
    program_pubkey: &Pubkey,
    connection: &RpcClient,
    update_data: &rhizo_types::SignedOnchainBytesUpdate,
) -> Result<(), Error> {
    let mut spinner = spinners::Spinner::new(spinners::Spinners::Dots8Bit, "Preparing Solana transaction..".into());
    let seed = &update_data.seed;
    let seed = format!("socb-{seed}");
    let (_, bump_seed) = ProgramPubkey::find_program_address(
        &[seed.as_bytes(),
          caller.pubkey().to_bytes().as_slice()],
          program_pubkey    
    );
    
    let created_pda = ProgramPubkey::create_program_address(&[seed.as_bytes(), caller.pubkey().to_bytes().as_slice(), &[bump_seed]], program_pubkey).unwrap();
   
    println!("OCB PDA: {:?}", created_pda);

    let update_request = rhizo_types::SignedOnchainBytesUpdate {
        seed: seed.to_owned(),
        bytes: update_data.bytes.to_owned(),
        bump_seed: Some(bump_seed),
    };

    let mut update_request_vec = 3u64.to_le_bytes().to_vec(); // Instruction marker for update ocb
    update_request_vec.extend(update_request.try_to_vec().expect("ocb update data can be serialized"));

    let request_update_instruction = Instruction {
        program_id: program_pubkey.to_owned(),
        accounts: vec![
            AccountMeta { pubkey: caller.pubkey(), is_signer: true, is_writable: true },
            AccountMeta { pubkey: created_pda, is_signer: false, is_writable: true },
            AccountMeta { pubkey: system_program::id(), is_signer: false, is_writable: false }
        ],
        data: update_request_vec
    };

    let message = Message::new(&[
                               request_update_instruction.clone(), 
    ], Some(&caller.pubkey()));

    //if false{
    
    let fee_lamports = connection.get_fee_calculator_for_blockhash(&connection.get_latest_blockhash().unwrap()).unwrap().unwrap().calculate_fee(&message);
    let rent_exemption = connection.get_minimum_balance_for_rent_exemption(request_update_instruction.data.len()).unwrap();
    spinner.stop_with_symbol("🗸");        
    let confirmation = dialoguer::Confirm::new()
        .with_prompt(format!("  Deploy route operation costs an estimated minimum of {:?} lamports + an additional {:?} to initialize a route keyed by {:?}, continue?", fee_lamports,rent_exemption, seed.clone()))
        .interact()
        .unwrap();

    if !confirmation {
        return Err(Error::new("Unable to submit transaction - cancelled due to user input"))
    }
    //}
    let mut spinner = spinners::Spinner::new(spinners::Spinners::Dots8Bit, "Sending Solana transaction..".into());
    
    let transaction =
        Transaction::new(&[caller], message, connection.get_latest_blockhash().expect("can fetch recent blockhash"));
   
    println!("{:?}", bincode::serialize(&transaction).unwrap().to_base64(b64::URL_SAFE));

    /*
    let try_transaction = connection.send_and_confirm_transaction(&transaction);
    if try_transaction.is_err() {
        println!("TransactionError {:?}", try_transaction);
        let transaction_error = try_transaction.err().unwrap().get_transaction_error().unwrap();
        let error_message = match transaction_error {
            TransactionError::InstructionError(index, InstructionError::Custom(error_code)) => {
                match (index, error_code) {
                    (_, 0u32) => "Developer has reached the smart-contract's configured route limit.".to_string(),
                    (_, 1u32) => "Nice try.".to_string(),
                    (_, 3u32) => "Not authorized to mutate bytes".to_string(),
                    (_,_) => { "Unsupported error code".to_string()}
                }
            }
            other => { other.to_string() }
        };
        return Err(Error::new(error_message.as_str()))
    }
    */
    spinner.stop_with_symbol("🗸");

    Ok(()) 
}

pub fn update_route_data(
    caller: &Keypair,
    program_pubkey: &Pubkey,
    connection: &RpcClient,
    route_data: &rhizo_types::RouteData,
    operation_byte: u8,
) -> Result<(), Error> {
    let mut spinner = spinners::Spinner::new(spinners::Spinners::Dots8Bit, "Preparing Solana transaction..".into());
    let account_seed_string = &route_data.route.clone();
 
    let account_seed_string = account_seed_string;
    let dev_routes_seed = b"_dev_routes";

    let (_, bump_seed) = ProgramPubkey::find_program_address(
        &[account_seed_string.as_bytes(),
          caller.pubkey().to_bytes().as_slice()],
          program_pubkey    
    );

    let (_, dev_routes_bump_seed) = ProgramPubkey::find_program_address(
        &[dev_routes_seed,
          caller.pubkey().to_bytes().as_slice()],
          program_pubkey    
    );

    let created_pda = ProgramPubkey::create_program_address(&[account_seed_string.as_bytes(), caller.pubkey().to_bytes().as_slice(), &[bump_seed]], program_pubkey).unwrap();
    let dev_routes_pda = ProgramPubkey::create_program_address(&[dev_routes_seed, caller.pubkey().to_bytes().as_slice(), &[dev_routes_bump_seed]], program_pubkey).unwrap();    

    let request_update_data = rhizo_types::RouteUpdate {
        route: account_seed_string.to_owned(),
        bump_seed: Some(dev_routes_bump_seed),
        operation: operation_byte,
    };

    let updated_route_data = rhizo_types::RouteData {
        route: account_seed_string.to_owned(),
        module_cid: route_data.module_cid,
        encodings: route_data.encodings.clone(),
        arguments: route_data.arguments.clone(),
        bump_seed: Some(bump_seed),
        cache_config: route_data.cache_config,
    };

    let mut request_update_data_vec = 1u64.to_le_bytes().to_vec(); // Instruction marker for route data    
    let mut update_data_vec = 0u64.to_le_bytes().to_vec(); // Instruction marker for route data

    request_update_data_vec.extend(request_update_data.try_to_vec().expect("request update can be serialized"));
    update_data_vec.extend(updated_route_data.try_to_vec().expect("route data can be serialized"));

    let request_update_instruction = Instruction {
        program_id: program_pubkey.to_owned(),
        accounts: vec![
            AccountMeta { pubkey: caller.pubkey(), is_signer: true, is_writable: true },
            AccountMeta { pubkey: dev_routes_pda, is_signer: false, is_writable: true },
            AccountMeta { pubkey: system_program::id(), is_signer: false, is_writable: false }
        ],
        data: request_update_data_vec
    };

    let update_instruction = Instruction {
        program_id: program_pubkey.to_owned(),
        accounts: vec![
            AccountMeta { pubkey: caller.pubkey(), is_signer: true, is_writable: true },
            AccountMeta { pubkey: created_pda, is_signer: false, is_writable: true },
            AccountMeta { pubkey: system_program::id(), is_signer: false, is_writable: false },
            AccountMeta { pubkey: dev_routes_pda, is_signer: false, is_writable: false },            
        ],
        data: update_data_vec
    };

    let balance = connection.get_balance(&created_pda).unwrap();

    let message = {
        if request_update_data.operation == 0u8 {

        Message::new(&[
            request_update_instruction.clone(), 
            update_instruction.clone()
        ], Some(&caller.pubkey()))
        } else {
        let mut data = 4u64.to_le_bytes().to_vec();
        data.extend(balance.to_le_bytes().to_vec());
        let delete_instruction = Instruction {
        program_id: program_pubkey.to_owned(),
        accounts: vec![
            AccountMeta { pubkey: caller.pubkey(), is_signer: true, is_writable: true },
            AccountMeta { pubkey: created_pda, is_signer: false, is_writable: true },
        ],
        data,
        };
            Message::new(&[
            request_update_instruction.clone(), 
            delete_instruction.clone()
        ], Some(&caller.pubkey()))
        }
    };

    //if false{
    
    let fee_lamports = connection.get_fee_calculator_for_blockhash(&connection.get_latest_blockhash().unwrap()).unwrap().unwrap().calculate_fee(&message);
    let rent_exemption = connection.get_minimum_balance_for_rent_exemption(update_instruction.data.len()).unwrap();
    spinner.stop_with_symbol("🗸");        
    let confirmation = dialoguer::Confirm::new()
        .with_prompt(format!("  Deploy route operation costs an estimated minimum of {:?} lamports + an additional {:?} to initialize a route keyed by {:?}, continue?", fee_lamports,rent_exemption, account_seed_string.clone()))
        .interact()
        .unwrap();

    if !confirmation {
        return Err(Error::new("Unable to submit transaction - cancelled due to user input"))
    }
    //}
    let mut spinner = spinners::Spinner::new(spinners::Spinners::Dots8Bit, "Sending Solana transaction..".into());
    
    let transaction =
        Transaction::new(&[caller], message, connection.get_latest_blockhash().expect("can fetch recent blockhash"));

    let try_transaction = connection.send_and_confirm_transaction(&transaction);
    if try_transaction.is_err() {
        println!("TransactionError {:?}", try_transaction);
        let transaction_error = try_transaction.err().unwrap().get_transaction_error().unwrap();
        let error_message = match transaction_error {
            TransactionError::InstructionError(index, InstructionError::Custom(error_code)) => {
                match (index, error_code) {
                    (_, 0u32) => "Developer has reached the smart-contract's configured route limit.".to_string(),
                    (_, 1u32) => "Nice try.".to_string(),
                    _ => "Unsupported InstructionError code".to_string()
                }
            }
            other => { other.to_string() }
        };
        println!("[ERROR]: {error_message}");
        return Err(Error::new(error_message.as_str()))
    }
    spinner.stop_with_symbol("🗸");
    Ok(()) 
    
}

