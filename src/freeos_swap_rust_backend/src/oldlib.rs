use ic_cdk::api::call;
use candid::CandidType;
use candid::Principal;
use candid::Nat;
use ic_cdk::api::management_canister::http_request::{http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,TransformContext, TransformFunc};
use serde::{Serialize, Deserialize};
use serde_json::{self, Value};
use std::str::FromStr;
use std::error::Error;

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct UserRecord {
    proton_account: String,
    ic_principal: String,
    amount: String,
    utc_time: String,
}

pub type Subaccount = [u8; 32];

#[derive(Clone, Serialize, Deserialize, CandidType, Debug)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}

#[derive(Clone, Deserialize, CandidType, Debug)]
pub struct Memo {
    pub memo: String,
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

pub type Tokens = Nat;

const ICRC1_LEDGER_CANISTER_ID: &str = "mxzaz-hqaaa-aaaar-qaada-cai";
static mut TRANSFER_FEE: &str = "100";

async fn get_table_rows(contract : String, start_utc_time: i64, end_utc_time : i64, num_rows : i32) -> Result<String, Box<dyn Error>> {
    let request_url : String = String::from("https://xxapi-xprnetwork-test.saltant.io/v1/chain/get_table_rows");

    let request_body: String = format!(
        r#"{{"json":true,"code":"{}","lower_bound":{},"upper_bound":{},"table":"swaps","scope":"freeosgov2","limit":{}}}"#,
        contract, start_utc_time, end_utc_time, num_rows
    );

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

    // prepare the request_body
    let json_utf8: Vec<u8> = request_body.into_bytes();
    let request_body_vec: Option<Vec<u8>> = Some(json_utf8);

    let request = CanisterHttpRequestArgument {
        url: request_url.clone(),
        method: HttpMethod::POST,
        body: request_body_vec,               //optional for request
        max_response_bytes: None, //optional for request
        transform: Some(TransformContext {
            // The "method" parameter needs to have the same name as the function name of your transform function
            function: TransformFunc(candid::Func {
                principal: ic_cdk::api::id(),
                method: "clean_dynamic_content".to_string(),
            }),
            // The "TransformContext" function does need a context parameter, it can be empty
            context: vec![],
        }),
        // transform: None,
        headers: request_headers,
    };

    //Note: in Rust, `http_request()` already sends the cycles needed
    //so no need for explicit Cycles.add() as in Motoko
    match http_request(request, 21_850_258_000).await {
        //4. DECODE AND RETURN THE RESPONSE

        //See:https://docs.rs/ic-cdk/latest/ic_cdk/api/management_canister/http_request/struct.HttpResponse.html
        Ok((response,)) => {
            let string_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            // ic_cdk::api::print(format!("About to return {:?}", string_body));

            Ok(string_body)
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            //Return the error as a string and end the method
            Err(message.into())
        }
    }

}

#[ic_cdk::update]
pub async fn autorun() -> () {
    let ledger_id: Principal = set_up_transfer_id();
    let foobar: u64 = 100;
    let working_transfer_fee = Tokens::from(foobar);
    let to_str = "aaaaa-aa";
    let to = Principal::from_str(to_str);

    let to_principal = to.map_err(|err| format!("Error parsing to principal: {}", err)).unwrap();

    let amount = 100000;
    let who_str = "aaaaa-aa";
    let who = Principal::from_str(who_str);

    let who_principal = who.map_err(|err| format!("Error parsing to principal: {}", err)).unwrap();

    let mut balance = balance_of(who_principal).await;
    let result = transfer(to_principal, amount, working_transfer_fee.clone()).await;
    balance = balance_of(who_principal).await;
    let mint_result = mint_tokens(to_principal, amount, working_transfer_fee.clone()).await;
}

#[ic_cdk::update]
pub fn set_up_transfer_id() -> Principal {
    let ledger_id = Principal::from_text("mxzaz-hqaaa-aaaar-qaada-cai").unwrap();
    println!("{}", ledger_id);
    ic_cdk::api::print(format!("The ledger id is: {}", ledger_id));
    ledger_id
}

#[ic_cdk::update]
pub async fn balance_of(who: Principal) -> Nat {
    let ledger_id = Principal::from_str("mxzaz-hqaaa-aaaar-qaada-cai").unwrap();
    let mut user_balance: Nat = candid::Nat::from(0u32);
    let transfer_account = Account {
        owner: who,
        subaccount: None,
    };
    let result: Result<(Nat,),  _> = call::call(ledger_id, "icrc1_balance_of", (transfer_account.clone(),)).await;

    match result {
        Ok((balance, )) => {
            ic_cdk::api::print(format!("Balance of {} is now {:#?}", transfer_account.owner, balance));
            user_balance = balance;
        }
        Err(err) => {
            eprintln!("Balance query failed: {:?}", err);
        }
    }
    
    ic_cdk::api::print(format!("Balance of {} is now {:#?}", transfer_account.owner, user_balance));
    return user_balance
}

#[ic_cdk::update]
pub async fn transfer(to: Principal, amount: u64, working_transfer_fee: Tokens) -> Result<(), String> {
    let ledger_id = Principal::from_str("mxzaz-hqaaa-aaaar-qaada-cai").unwrap();
    let transfer_account = Account {
        owner: to,
        subaccount: None,
    };
    let transfer_info = TransferArg {
        from_subaccount: None,
        to: transfer_account,
        fee: Some(working_transfer_fee),
        created_at_time: None,
        memo: None,
        amount: Tokens::from(amount),
    };
    let result: Result<(), _> = call::call(ledger_id, "icrc1_transfer", (transfer_info.clone(), )).await;
    ic_cdk::api::print(format!("{} has been moved to {}", transfer_info.amount, ledger_id));
    result.map_err(|err| format!("Transfer failed: {:?}", err))
}

#[ic_cdk::update]
pub async fn mint_tokens(to: candid::Principal, amount: u64, working_transfer_fee: Tokens) -> Nat {

    // Call transfer method on icrc1_ledger canister
    let balance = transfer(to, amount, working_transfer_fee).await;
    

    // Call balance_of method on icrc1_ledger canister
    let balance = balance_of(to).await;
    println!("Balance of {:#?}: {:#?}", to, balance);

    return balance
}

#[ic_cdk::update]
pub async fn main() -> Principal {
    let working_transfer_id = set_up_transfer_id();

    let foobar: u64 = 100;
    let working_transfer_fee = Tokens::from(foobar);
    let to = Principal::from_str("mxzaz-hqaaa-aaaar-qaada-cai").unwrap();
    let amount = 100000;
    let who = Principal::from_text("mxzaz-hqaaa-aaaar-qaada-cai").unwrap();
    let mut balance = balance_of(who).await;
    let result = transfer(to, amount, working_transfer_fee.clone()).await;
    balance = balance_of(who).await;
    let mint_result = mint_tokens(to, amount, working_transfer_fee.clone()).await;
    return working_transfer_id;
}

#[ic_cdk::update]
pub async fn create_user_record() -> (String, UserRecord) {
    let request_url : String = String::from("https://api-xprnetwork-test.saltant.io/v1/chain/get_table_rows");
    let request_body : String = String::from(r#"{"json":true,"code":"freeosgov2","lower_bound":1726732990,"upper_bound":1726735767,"table":"swaps","scope":"freeosgov2","limit":100}"#);

    let host = request_url.split('/').nth(2).unwrap_or_default().to_string();
    let idempotency_key = "d83jf920djc8shf92j8fhs93d82fhs94";
    
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
        HttpHeader {
            name: "Idempotency-Key".to_string(),
            value: idempotency_key.to_string(),
        },
    ];

    // Prepare the request_body
    let json_utf8: Vec<u8> = request_body.into_bytes();
    let request_body_vec: Option<Vec<u8>> = Some(json_utf8);

    let request = CanisterHttpRequestArgument {
        url: request_url.clone(),
        method: HttpMethod::POST,
        body: request_body_vec,   // Optional for request
        max_response_bytes: None, // Optional for request
        transform: Some(TransformContext {
            // The "method" parameter needs to have the same name as the function name of your transform function
            function: TransformFunc(candid::Func {
                principal: ic_cdk::api::id(),
                method: "clean_dynamic_content".to_string(),
            }),
            // The "TransformContext" function does need a context parameter, it can be empty
            context: vec![],
        }),
        // transform: None,
        headers: request_headers,
    };

    //Note: in Rust, `http_request()` already sends the cycles needed, so there is no need for explicit Cycles.add() as in Motoko
    match http_request(request, 21_850_258_000).await {
        //4. DECODE AND RETURN THE RESPONSE

        //See:https://docs.rs/ic-cdk/latest/ic_cdk/api/management_canister/http_request/struct.HttpResponse.html
        Ok((response,)) => {
            // get the raw response body for debugging purposes
            // ic_cdk::api::print(format!("Raw response body: {:?}", response.body));

            //if successful, `HttpResponse` has this structure:
            // pub struct HttpResponse {
            //     pub status: Nat,
            //     pub headers: Vec<HttpHeader>,
            //     pub body: Vec<u8>,
            // }

            //You need to decode that Vec<u8> that is the body into readable text.
            //To do this:
            //  1. Call `String::from_utf8()` on response.body
            //  3. Use a switch to explicitly call out both cases of decoding the Blob into ?Text
            let string_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            ic_cdk::api::print(format!("{:?}", string_body));

            // SERDE
            // Parse the JSON string
            let str_body: &str = string_body.as_str();
            let mut new_user_record = UserRecord {
                proton_account: String::from(""),
                ic_principal: String::from(""),
                amount: String::from(""),
                utc_time: 0.to_string(),
            };
            match serde_json::from_str::<Value>(str_body) {
                Ok(parsed) => {
                    // Access the second record in the array and retrieve the Amount
                    if let Some(amount) = parsed["rows"][1]["proton_account"].as_str() {
                        ic_cdk::api::print(format!("Amount from the second record: {}", amount));
                        new_user_record.proton_account = amount.to_string();
                    } else {
                        ic_cdk::api::print(format!("Failed to retrieve proton_account from the second record"));
                    }
                }
                Err(e) => {
                    ic_cdk::api::print(format!("Failed to parse response body: {}", e));
                }
            }
            match serde_json::from_str::<Value>(str_body) {
                Ok(parsed) => {
                    // Access the second record in the array and retrieve the Amount
                    if let Some(amount) = parsed["rows"][1]["ic_principal"].as_str() {
                        ic_cdk::api::print(format!("Amount from the second record: {}", amount));
                        new_user_record.ic_principal = amount.to_string();
                    } else {
                        ic_cdk::api::print(format!("Failed to retrieve 'ic_principal' from the second record"));
                    }
                }
                Err(e) => {
                    ic_cdk::api::print(format!("Failed to parse response body: {}", e));
                }
            }
            match serde_json::from_str::<Value>(str_body) {
                Ok(parsed) => {
                    // Access the second record in the array and retrieve the Amount
                    if let Some(amount) = parsed["rows"][1]["amount"].as_str() {
                        ic_cdk::api::print(format!("Amount from the second record: {}", amount));
                        new_user_record.amount = amount.to_string();
                    } else {
                        ic_cdk::api::print(format!("Failed to retrieve 'amount' from the second record"));
                    }
                }
                Err(e) => {
                    ic_cdk::api::print(format!("Failed to parse response body: {}", e));
                }
            }
            match serde_json::from_str::<Value>(str_body) {
                Ok(parsed) => {
                    // Access the second record in the array and retrieve the Amount
                    if let Some(amount) = parsed["rows"][1]["utc_time"].as_u64() {
                        ic_cdk::api::print(format!("Amount from the second record: {}", amount));
                        new_user_record.utc_time = amount.to_string();
                    } else {
                        ic_cdk::api::print(format!("Failed to retrieve 'utc_time' from the second record"));
                    }
                }
                Err(e) => {
                    ic_cdk::api::print(format!("Failed to parse response body: {}", e));
                }
            }
            // END SERDE
        
            //The API response will looks like this:
            // { successful: true }

            //Return the body as a string and end the method
            let result: String = format!(
                "{}", str_body
            );
            ic_cdk::api::print(format!("{:#?}", new_user_record));
            // let working_object = new_user_record;
            let new_user_record = display_working_proton(new_user_record);
            let new_user_record = display_working_principal(new_user_record.0);
            let new_user_record = display_working_amount(new_user_record.0);
            let new_user_record = display_working_utc(new_user_record.0);
            return (result, new_user_record.0)
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            //Return the error as a string and end the method
            let empty_user_record = UserRecord {
                proton_account: String::from(""),
                ic_principal: String::from(""),
                amount: String::from(""),
                utc_time: String::from(""),
            };
            return (message, empty_user_record)
        }
    }

}

#[ic_cdk::query]
fn display_working_proton(working_object: UserRecord) -> (UserRecord, String) {
    let string_to_display: &str = &working_object.proton_account.clone();
    ic_cdk::api::print(format!("{:#?}", string_to_display));
    println!("{}", string_to_display);
    return (working_object, string_to_display.to_string())
}

#[ic_cdk::query]
fn display_working_principal(working_object: UserRecord) -> (UserRecord, String) {
    let string_to_display = working_object.ic_principal.clone();
    ic_cdk::api::print(format!("{:#?}", string_to_display));
    println!("{}", string_to_display);
    return (working_object, string_to_display.to_string())
}

#[ic_cdk::query]
fn display_working_amount(working_object: UserRecord) -> (UserRecord, String) {
    let string_to_display = working_object.amount.clone();
    ic_cdk::api::print(format!("{:#?}", string_to_display));
    println!("{}", string_to_display);
    return (working_object, string_to_display.to_string())
}

#[ic_cdk::query]
fn display_working_utc(working_object: UserRecord) -> (UserRecord, String) {
    let string_to_display = working_object.utc_time.clone();
    ic_cdk::api::print(format!("{:#?}", string_to_display));
    println!("{}", string_to_display);
    return (working_object, string_to_display.to_string())
}

#[ic_cdk::query]
fn clean_dynamic_content(args: TransformArgs) -> HttpResponse {
    let mut response = args.response;

    // Filter out the 'Date' header from the headers
    //response.headers.retain(|header| header.name != "Date" && header.name != "CF-RAY" && header.name != "Report-To");
    response.headers.clear();
    
    // Return the cleaned response
    response
}

