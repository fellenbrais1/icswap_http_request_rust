// Experimental crypto-currency bridge code

// PURPOSE
// This code sets up the structs etc. needed to run a http GET request from the FREEOS PROTON server, fetching user records with their principals and amounts they wish to convert and then performing an auto-minting of an amount of FREEOS on the Internet Computer while the PROTON side handles burning of an equivalent amount of FREEOS on PROTON. This auto-record generation via http request and minting can be set to run based on an INTERVAL variable in seconds. A series of messages can be generated from this code to indicate if parts have succeeded or failed.

// OPERATION
// Follow the instructions in the README.md file to deploy correctly. Do not run dfx deploy before the shell files have been run otherwise the canisters won't know which one is the MINTER etc. and this will stop calls to the icrc1_ledger canister from working correctly.

// TO ADD
// Some of this code relies on .expect() which is sub-optimal as it can cause canister trapping and this is not good for a system meant to run periodicially. Error handling could be added to these instances to gracefully handle any potential errors.
// An error_log hashmap was to be created, which would hold the user records generated by the http request and any errors encountered while trying to extract information from the record or minting the amount etc. This would mean in times of potential error, we can clear things up by taking a look at this error record.
// Error handling was going to basically add a message to the log and then skip over the problem record, so the rest of the auto-minting could occur, at the moment this does not happen.
// A variable was meant to be set up to see how many records were successfully processed and where through the table of user records on the frontend the program got up to. Then, the http requests bounds could be changed to only look for new records from beyond this successful point.
// This also needs to be connected to the frontend, though I imagine that might be more simple, only needing the PROTON principal, the IC principal, the amount and the unix time from the frontend to be fed into this canister to generate the http GET request.

// KEYWORD 'TODO'
// I have marked areas of problems and actions to be taken with the keyword TODO, search the file with this keyword to find all points where action needs to be taken

// PITFALLS
// Apologies for the unfinished state of the code. 
// Please be wary of using new crates as many are silently unsupported by the Internet Computer platform, if you get a cryptic error when deploying etc. involving something called 'bindgen' you have likely used an invalid crate and will have to remove it. 'chrono' is an example of an unsupported crate. Currently there is no way around this that I know of.
// As mentioned above, amounts given to the icrc1_ledger canister have to be in Nat format, so all floating point numbers have to be converted to Nats before minting etc. can occur.
// Please check the README.md, set_env.sh, and deploy_icrc1.sh files to configure things to your liking. You will have to set up a dfx identity called 'archive_controller' to nake it work but this can have any password etc., the name just has to match.

// CONTACT
// As you will no doubt see from the code, my knowledge of Rust and the Internet Computer has some Grand Canyon-sized gaps in it. However, if you need me to explain anything I have tried to do here or how things are supposed to work, contact me on Discord at michaelmccann88 or email me at fellenbrais@gmail.com and I will try to help wherever I can. All the best and good luck.

// All crates currently used by the program
use candid::types::principal::PrincipalError;
use candid::{CandidType, Nat, Principal};
use ic_cdk::api::management_canister::http_request::{http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,TransformContext, TransformFunc};
use ic_cdk::api::call::{self};
use ic_cdk::spawn;
use serde::{Serialize, Deserialize};
use serde_json::{self, Value};
use std::vec;
use std::time::Duration; 
use std::collections::HashMap;
use ic_cdk::api;

// Custom structs etc. needed for the icrc1_ledger canister's functions and other uses in this canister
#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct UserRecord {
    proton_account: String,
    ic_principal: String,
    amount: f32,
    utc_time: u64,
}

pub type Subaccount = [u8; 32];
pub type Tokens = Nat;

const ICRC1_LEDGER_CANISTER_ID: &str = "mxzaz-hqaaa-aaaar-qaada-cai";
const INTERVAL: Duration = Duration::from_secs(60 * 60 * 24); // Seconds in one day 
// const INTERVAL: Duration = Duration::from_secs(60); // Test amount = one minute

// TODO - Some logic in the code to convert from decimals to whole numbers and vice-versa might need some work, the problem is that the Internet Computer can only handle amounts in Nat, which means decimal amounts have to be converted to whole numbers etc. Check the conversion functions
// Converting FREEOS decimal point values to Nats we can use
// 1 unit of FREEOS = 0.0001
// Therefore, multiply all user inputs by 10,000 to get a usable number for this program

// This isn't used anywhere in the program and is more for reference
// TODO - Could be removed?
#[allow(dead_code)]
const MINTER_ID: &str = "bd3sg-teaaa-aaaaa-qaaba-cai";

#[derive(Clone, Deserialize, CandidType, Debug)]
pub struct Memo {
    pub memo: String,
} 

#[derive(Clone, Serialize, Deserialize, CandidType, Debug, Copy)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}

#[derive(Clone, CandidType, Debug)]
pub struct TransferArg {
    pub from_subaccount: Option<Subaccount>,
    pub to: Account,
    pub fee: Option<Tokens>,
    pub created_at_time: Option<u64>,
    pub memo: Option<Memo>,
    pub amount: Tokens,
}

pub struct TransferResult {
    pub ok: BlockIndex,
    pub err: TransferError,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq)]
pub struct ProtonResult {
    pub ok: Option<String>,
    pub err: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, CandidType, Debug)]
pub enum TransferError {
    BadFee {
        expected_fee: Tokens,
    },
    BadBurn {
        min_burn_amount: Tokens,
    },
    InsufficientFunds {
        balance: Tokens,
    },
    TooOld,
    CreatedInFuture {
        ledger_time: u64,
    },
    TemporarilyUnavailable,
    Duplicate {
        duplicate_of: BlockIndex,
    },
    GenericError {
        error_code: Nat,
        message: String,
    },
}

pub type BlockIndex = Nat;

// Meant to be used to set up initial variables when deploying the canister
// TODO Doesn't seem to do anything yet, but duration of autocall could potentially be set here instead of in post_upgrade()
#[ic_cdk::init]  
async fn init() {  
    ic_cdk::api::print("Initializing Canister");
}

// Meant to run after upgrade to set the INTERVAL amount for the timer_callback() fucntion
// TODO - Supposed to call init() but I don't think it is working as intended presently - problem with init()?
#[ic_cdk::post_upgrade]  
async fn post_upgrade() {  
    let _timer_id = ic_cdk_timers::set_timer_interval(INTERVAL, timer_callback);
    init().await;
}

// Based on the duration, calls auto_call() function
// TODO - Supposed to run automatically, but sometimes will only start running once auto_call() has been called once manually - problem with init()/ post_upgrade()?
fn timer_callback() {
    // Spawn the async work without blocking
    spawn(async {
        auto_call().await;
    });
}

// Automatically generates the http GET request and minting process by calling create_user_record()
// Called by timer_callback()
#[ic_cdk::update]
pub async fn auto_call() -> () {  
    ic_cdk::api::print("auto_call() - Automatic function call running");
    let result = create_user_record().await;
    ic_cdk::api::print(format!("{:?}", result));
}

// Function to calculate the Unix time stamp
// Called by current_unix_time()
fn get_unix_time_seconds() -> u64 {
    let time_nanos = api::time();
    time_nanos / 1_000_000_000
}

// Function to generate Unix time stamps and returning them as a number
// Can be called from Candid and other functions
// TODO - This was intended to be used when logging error messages etc, so call it to create time stamps when you need them
#[ic_cdk::query]
pub fn current_unix_time() -> u64 {
    get_unix_time_seconds()
}

// Function used to convert floating point numbers to whole numbers by effectively removing a decimal point entirely
// Called by functions that need whole numbers for minting etc. like create_user_record()
// TODO Could be merged with undecimalize_freeos_amount()?
#[ic_cdk::update]
pub fn whole_amount_from_decimal(amount: f64) -> u64 {
    ic_cdk::api::print("whole_amount_from_decimal() - Conversion to whole number amount from decimal");
    let print_amount = undecimilaze_freeos_amount(amount);
    ic_cdk::api::print(format!("Original amount was: {:?}, new amount is: {:?}", amount, print_amount));
    return print_amount
}

// Multiplies a number by 10_000 to effectively move all 4 decimal places of a FREEOS above 0
// Called by whole_amount_by_decimal()
#[ic_cdk::update]
fn undecimilaze_freeos_amount(amount: f64) -> u64 {
    let amount_to_convert: u64 = (amount * 10_000.0) as u64;
    let new_amount = amount_to_convert;
    return new_amount
}

// Function used to convert whole numbers to floating point numbers by moving the decimal place 4 places into the number
// Called by functions that need to display floating point numbers like balance_of()
// TODO Could be merged with decimalize_freeos_amount()
#[ic_cdk::update]
pub fn decimal_amount_from_whole(amount: u64) -> f64 {
    ic_cdk::api::print("decimal_amount_from_whole() - Conversion to decimal amount from whole number");
    let print_amount = decimilaze_freeos_amount(amount);
    ic_cdk::api::print(format!("Original amount was: {:?}, new amount is: {:?}", amount, print_amount));
    return print_amount
}

// Divides a number by 10_000 to effectively move all 4 decimal places of a FREEOS under 0
// Called by decimal_amount_from_whole()
#[ic_cdk::update]
fn decimilaze_freeos_amount(amount: u64) -> f64 {
    let new_amount: f64 = amount as f64 / 10_000.0;
    return new_amount
}

// Checks if an IC principal retrieved from the http GET request is valid or not
// Called by create_user_record()
fn check_principal(principal_to_check: &str) -> Result<Principal, PrincipalError> {
    match Principal::from_text(principal_to_check) {
        Ok(principal) => Ok(principal),
        Err(error) => {
            ic_cdk::println!("Error processing item: {}", error);
            Err(error)
        }
    }
}

// Checks if a PROTON principal retrieved from the http GET request is valid or not
// Called by create_user_record()
// TODO Could use some tidying, change the name
pub fn proton_account_to_check(account_to_check: &str) -> ProtonResult {
    let is_valid_proton_account = is_valid_proton_account(account_to_check);

    if is_valid_proton_account {
        ProtonResult { ok: Some(account_to_check.to_string()), err: None }
    } else {
        ProtonResult { ok: None, err: Some("Invalid Proton account".to_string()) }
    }
}

// Returns a bool depending on whether the PROTON account &str is empty or not
// Called by proton_account_to_check()
// TODO Could use some tidying, change the name
fn is_valid_proton_account(account: &str) -> bool {
    if account.is_empty() {
        return false
    } else {
        return true
    }
}

// Processes the http GET request to create a user record and then auto-mints the amounts in the record to the IC principal specified
// Called by auto_call()
// TODO - This is where the bulk of the error-handling code needs to be added as well as recording the errors in an error log
#[ic_cdk::update]
pub async fn create_user_record() -> String {
    let request_url : String = String::from("https://api-xprnetwork-test.saltant.io/v1/chain/get_table_rows");
    let request_body : String = String::from(r#"{"json":true,"code":"freeosgov2","lower_bound":1730065553,"upper_bound":1730065936,"table":"swaps","scope":"freeosgov2","limit":100}"#);

    let host = request_url.split('/').nth(2).unwrap_or_default().to_string();
    
    let request_headers = vec![
        HttpHeader {
            name: "Host".to_string(),
            value: host,
        },
        HttpHeader {
            name: "User-Agent".to_string(),
            value: "IC-Agent".to_string(),
        },
        HttpHeader {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        },
    ];

    let json_utf8: Vec<u8> = request_body.into_bytes();
    let request_body_vec: Option<Vec<u8>> = Some(json_utf8);

    let request = CanisterHttpRequestArgument {
        url: request_url.clone(),
        method: HttpMethod::POST,
        body: request_body_vec,
        max_response_bytes: None,
        transform: Some(TransformContext {

            function: TransformFunc(candid::Func {
                principal: ic_cdk::api::id(),
                method: "clean_dynamic_content".to_string(),
            }),

            context: vec![],
        }),

        headers: request_headers,
    };

    //Note: in Rust, `http_request()` already sends the cycles needed, so there is no need for explicit Cycles.add() as in Motoko
    match http_request(request, 21_850_258_000).await {

        Ok((response,)) => {

            let string_body = String::from_utf8(response.body).expect("Transformed response is not UTF-8 encoded.");
            ic_cdk::api::print(format!("{:?}", string_body));

            // SERDE
            // TODO - The operation_error code was my attempt at creating strings of errors that appear and then the next step would be to log these to a hashmap or other datastructure that can act as a record that we can check in case anything goes wrong, in the case of an error, the code should skip this row and keep on minting etc.
            let str_body: &str = string_body.as_str();
            match serde_json::from_str::<Value>(str_body) {
                Ok(parsed) => {
                    if let Some(rows) = parsed["rows"].as_array() {
                        let mut counter = 0;

                        let mut usable_principal: Option<Principal> = None;
                        let mut error_map: HashMap<String, String> = HashMap::new();
                        let operation_error: String = "".to_string();
                        match check_principal("aaaaa-aa") {
                            Ok(principal) => {
                                usable_principal = Some(principal);
                            }
                            Err(error) => {
                                let operation_error = error.to_string();
                                error_map.insert("IC_principal".to_string(), operation_error);
                                ic_cdk::println!("Error parsing principal: {}", error);
                            }
                        }

                        let mut amount_number_raw: u64 = 0;
                        for row in rows {
                            if let Some(proton_account) = row["proton_account"].as_str() {
                                ic_cdk::api::print(format!("Proton Account from record {}: {}", counter, proton_account));
                            } else {
                                let operation_error = "Failed to retrieve 'proton_account' from the row".to_string();
                                ic_cdk::api::print(operation_error);
                            }
                            if let Some(ic_principal) = row["ic_principal"].as_str() {
                                ic_cdk::api::print(format!("IC Principal from record {}: {}", counter, ic_principal));
                            
                                match check_principal(ic_principal) {
                                    Ok(principal) => {
                                        usable_principal = Some(principal);
                                    }
                                    Err(error) => {
                                        ic_cdk::println!("Error parsing principal: {}", error);
                                    }
                                }
                            } else {
                                ic_cdk::api::print(format!("Failed to retrieve 'ic_principal' from the row"));
                            }
                            if let Some(amount) = row["amount"].as_str() {
                                let index = amount.find(" ");

                                if let Some(index) = index {
                                    let amount_str = &amount[..index];
                                    ic_cdk::api::print(format!("{}", amount_str));

                                    match amount_str.parse::<f64>() {
                                        Ok(amount_number) => {
                                            // Successful parsing, use amount_number
                                            ic_cdk::api::print(format!("Parsed amount: {}", amount_number));
                                            let amount_number = whole_amount_from_decimal(amount_number);
                                            amount_number_raw = amount_number;
                                            ic_cdk::api::print(format!("Amount from record {}: {}", counter, amount_number));
                                        }
                                        Err(e) => {
                                            // Handle parsing error, e.g., log an error or return an error value
                                            ic_cdk::api::print(format!("Parsing error: {}", e));
                                        }
                                    }
                                    // let amount_number: u64 = amount_str.parse().unwrap();
                                } else {
                                    ic_cdk::api::print(format!("Failed to retrieve 'amount' from the row"));
                                }
                            } else {
                                ic_cdk::api::print(format!("Failed to retrieve 'amount' from the row"));
                            }
                            if let Some(utc_time) = row["utc_time"].as_u64() {
                                ic_cdk::api::print(format!("UTC Time from record {}: {}", counter, utc_time));
                            } else {
                                ic_cdk::api::print(format!("Failed to retrieve 'utc_time' from the row"));
                            }
                            // Calling the minting process for this record
                            let amount_number = amount_number_raw;

                            // INTER CANISTER CALL
                            match usable_principal {
                                Some(principal) => {
                                    let mint_result = mint_amount(principal, amount_number).await;
                                    ic_cdk::api::print(format!("MINT RESULT is: {:?}, AMOUNT MINTED: {:?}", mint_result, amount_number));
                                }
                                None => {
                                    ic_cdk::api::print(format!("Mint processing failed due to {}", operation_error));
                                }
                            }
                            counter += 1;
                        }
                    } else {
                        ic_cdk::api::print(format!("Failed to retreive 'rows' from the response"));
                    }
                }
                Err(e) => {
                    ic_cdk::api::print(format!("Failed to parse response body: {}", e));
                }
            }
            // END SERDE
    
            let result: String = format!(
                "{}", str_body
            );

            return result
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            return message
        }
    }
}

// Mints the specified amount of FREEOS to the specified principal
// Called by create_user_record()
#[ic_cdk::update]
pub async fn mint_amount(recipient: Principal, amount: u64) -> (String, u64) {
    ic_cdk::api::print("MINT CODE IS RUNNING");
    
    let recipient_principal = recipient;
    let amount_number = amount;

    // INTER CANISTER CALL
    transfer(recipient_principal, amount_number).await;

    // INTER CANISTER CALL
    let balance_result = balance_of(recipient).await;

    let new_balance: u64 = balance_result.1;
    let print_string = format!("Balance of {} is now: {}", recipient, new_balance);
    return (print_string, new_balance)
}

// Function to clean the dynamic content of the http GET request after each request
// Called by create_user_record()
#[ic_cdk::query]
fn clean_dynamic_content(args: TransformArgs) -> HttpResponse {
    let mut response = args.response;
    
    response.headers.clear();
    
    response
}

// Function that can be used to check the balance of a principal
// Can be called in Candid or by mint_amount()
#[ic_cdk::update]
pub async fn balance_of(principal_to_check: Principal) -> (String, u64) {
    ic_cdk::api::print("BALANCE OF CODE IS RUNNING");
    let ledger_id = Principal::from_text(ICRC1_LEDGER_CANISTER_ID).unwrap();
    let transfer_account = Account {
        owner: principal_to_check,
        subaccount: None,
    };
    let result: Result<(u128,),  _> = call::call(ledger_id, "icrc1_balance_of", (transfer_account,)).await;
    
    match result {
        Ok((balance, )) => {
            let user_balance: f64 = balance as f64;
            let print_string = format!("Balance of {}: {:.4}", &transfer_account.owner, user_balance);
            ic_cdk::api::print(format!("{}", print_string));
            let user_balance = whole_amount_from_decimal(user_balance);
            return (print_string, user_balance)
        }
        Err(err) => {
            let print_string = format!("Balance query failed: {:?}", err);
            eprintln!("{}", print_string);
            let user_balance: u64 = u64::from(0u64);
            return (print_string, user_balance)
        }
    }
}

// Can be used to transfer an amount of tokens into a principal
// Can be called using Candid
#[ic_cdk::update]
pub async fn transfer(recipient: Principal, amount: u64) -> (String, u64) {
    ic_cdk::api::print("TRANSFER CODE IS RUNNING");
    let ledger_id = Principal::from_text(ICRC1_LEDGER_CANISTER_ID).unwrap();
    let transfer_account = Account {
        owner: recipient,
        subaccount: None,
    };
    let transfer_info = TransferArg {
        from_subaccount: None,
        to: transfer_account,
        fee: None,
        created_at_time: None,
        memo: None,
        amount: Nat::from(amount),
    };
    // INTER CANISTER CALL
    let call_result: Result<(Result<BlockIndex, TransferError>,), _> = call::call(ledger_id, "icrc1_transfer", (transfer_info, )).await;
    
    match call_result {
        Ok((inner_result,)) => {
            match inner_result {
                Ok(block_index) => {
                    // INTER CANISTER CALL
                    let balance = balance_of(recipient).await;

                    let balance_to_print = balance.1;
                    let print_string = format!("Transfer successful for {} with block index: {}, New balance is now: {:?}", &transfer_account.owner, block_index.to_string(), balance_to_print);
                    ic_cdk::api::print(&print_string);
                    return (print_string, balance_to_print)
                }
                Err(transfer_error) => {
                    let unchanged_balance = balance_of(recipient).await;
                    let balance_to_print = unchanged_balance.1;
                    let print_string = format!("Transfer failed with error: {:?}, balance of {} remains at: {}", transfer_error, recipient, balance_to_print);
                    ic_cdk::api::print(&print_string);
                    return (print_string, balance_to_print)
                }
            }
        }
        Err(err) => {
            let unchanged_balance = balance_of(recipient).await;
            let balance_to_print = unchanged_balance.1;
            let print_string = format!("Inter-canister call failed: {:?}, balance of {} remains at: {}", err, recipient, balance_to_print);
            ic_cdk::api::print(&print_string);
            return (print_string, balance_to_print)
        }
    }
}