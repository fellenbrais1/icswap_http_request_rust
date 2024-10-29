use candid::{CandidType, Nat, Principal};
use ic_cdk::api::management_canister::http_request::{http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,TransformContext, TransformFunc};
use ic_cdk::api::call::{self};
use ic_cdk::spawn;
use serde::{Serialize, Deserialize};
use serde_json::{self, Value};
use std::vec;
use std::time::Duration; 

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
// const INTERVAL: Duration = Duration::from_secs(60 * 60 * 24); // Seconds in one day 
const INTERVAL: Duration = Duration::from_secs(60); // Test amount 

// Converting FREEOS decimal point values to Nats we can use -- in progress
// 1 unit of FREEOS = 0.0001
// Therefore, multiply all user inputs by 10,000 to get a usable number for this program

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

#[ic_cdk::post_upgrade]  
fn post_upgrade() {  
    let _timer_id = ic_cdk_timers::set_timer_interval(INTERVAL, timer_callback);
}

fn timer_callback() {
    // Spawn the async work without blocking
    spawn(async {
        ring().await;
    });
}

#[ic_cdk::update]
pub async fn ring() -> () {  
    ic_cdk::api::print("Rust Timer Ring!");
    let result = create_user_record().await;
    ic_cdk::api::print(format!("{:?}", result));
}

#[ic_cdk::init]  
async fn init() {  
    ic_cdk::api::print("Initializing Canister");
}

#[ic_cdk::update]
fn undecimilaze_freeos_amount(amount: f64) -> u64 {
    ic_cdk::api::print("This code is running");
    let amount_to_convert: u64 = (amount * 10_000.0) as u64;
    let new_amount = amount_to_convert;
    return new_amount
}

#[ic_cdk::update]
fn decimilaze_freeos_amount(amount: u64) -> f64 {
    ic_cdk::api::print("This code is running");
    let new_amount: f64 = amount as f64 / 10_000.0;
    return new_amount
}

#[ic_cdk::update]
pub fn whole_amount_from_decimal(amount: f64) -> u64 {
    ic_cdk::api::print("This code is running - Whole amount from decimal");
    let print_amount = undecimilaze_freeos_amount(amount);
    ic_cdk::api::print(format!("Original amount was: {:?}, new amount is: {:?}", amount, print_amount));
    return print_amount
}

#[ic_cdk::update]
pub fn decimal_amount_from_whole(amount: u64) -> f64 {
    ic_cdk::api::print("This code is running - Decimal amount from whole number");
    let print_amount = decimilaze_freeos_amount(amount);
    ic_cdk::api::print(format!("Original amount was: {:?}, new amount is: {:?}", amount, print_amount));
    return print_amount
}

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

        //See:https://docs.rs/ic-cdk/latest/ic_cdk/api/management_canister/http_request/struct.HttpResponse.html
        Ok((response,)) => {
            // ic_cdk::api::print(format!("Raw response body: {:?}", response.body));

            let string_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            ic_cdk::api::print(format!("{:?}", string_body));

            // SERDE
            let str_body: &str = string_body.as_str();
            match serde_json::from_str::<Value>(str_body) {
                Ok(parsed) => {
                    if let Some(rows) = parsed["rows"].as_array() {
                        let mut counter = 0;
                        let mut transfer_principal: Principal = Principal::from_text("aaaaa-aa").expect("Failed to read line");
                        let mut amount_number_raw: u64 = 0;
                        // let amount_number = Nat::from(amount_number_raw);
                        for row in rows {
                            if let Some(proton_account) = row["proton_account"].as_str() {
                                ic_cdk::api::print(format!("Proton Account from record {}: {}", counter, proton_account));
                            } else {
                                ic_cdk::api::print(format!("Failed to retrieve 'proton_account' from the row"));
                            }
                            if let Some(ic_principal) = row["ic_principal"].as_str() {
                                ic_cdk::api::print(format!("IC Principal from record {}: {}", counter, ic_principal));
                                transfer_principal = Principal::from_text(ic_principal).expect("Failed to read line");
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

                            // Debug statements
                            ic_cdk::api::print(format!("AMOUNT NUMBER RAW IS: {}", amount_number));
                            ic_cdk::api::print(format!("AMOUNT NUMBER IS: {}", amount_number));

                            // INTER CANISTER CALL
                            let mint_result = mint_amount(transfer_principal, amount_number).await;
                            ic_cdk::api::print(format!("MINT RESULT is: {:?}, AMOUNT MINTED: {:?}", mint_result, amount_number));
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

#[ic_cdk::query]
fn clean_dynamic_content(args: TransformArgs) -> HttpResponse {
    let mut response = args.response;
    
    response.headers.clear();
    
    response
}

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