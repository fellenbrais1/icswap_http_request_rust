use ic_cdk::api::call;
use candid::CandidType;
use candid::Blob;
use candid::Principal;
use ic_cdk::api::management_canister::http_request::{http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,TransformContext, TransformFunc};
use serde::{Serialize, Deserialize};
use serde_json::{self, Value};

#[derive(Serialize, Deserialize, CandidType, Debug)]
struct UserRecord {
    proton_account: String,
    ic_principal: String,
    amount: String,
    utc_time: String,
}

type Tokens = u64;

#[derive(CandidType)]
struct MyData {
    blob_field: Blob,
}

struct Account {
    owner: Principal,
    subaccount: opt SubAccount,
}

#[derive(CandidType, Deserialize, Serialize)]
struct TransferArg {
    from_subaccount: opt SubAccount,
    to: Account,
    amount: Tokens,
    fee: opt Tokens,
    memo: opt blob,
    created_at_time: opt Timestamp,
};

const ICRC1_LEDGER_CANISTER_ID: &str = "mxzaz-hqaaa-aaaar-qaada-cai";
// static megastring: String = ICRC1_LEDGER_CANISTER_ID.to_string();

#[ic_cdk::update]
pub fn main() -> Principal {
    let working_transfer_id = set_up_transfer_id();
    return working_transfer_id
}

#[ic_cdk::update]
pub fn set_up_transfer_id() -> Principal {
    let working_transfer_id = Principal::from_text(ICRC1_LEDGER_CANISTER_ID).unwrap();
    println!("{}", working_transfer_id);
    ic_cdk::api::print(format!("The working transfer id is: {}", working_transfer_id));
    working_transfer_id
}

#[ic_cdk::update]
pub async fn transfer(to: candid::Principal, amount: u64) -> Result<(), String> {
    let working_transfer_id = Principal::from_text(ICRC1_LEDGER_CANISTER_ID).unwrap();
    let result: Result<(), _> = call::call(working_transfer_id, "transfer", (to, amount)).await;
    result.map_err(|err| format!("Transfer failed: {:?}", err))
}

#[ic_cdk::query]
pub async fn balance_of(who: Principal) -> Result<(), String> {
    let working_transfer_id = Principal::from_text(ICRC1_LEDGER_CANISTER_ID).unwrap();
    let result: Result<(), _> = call::call(working_transfer_id, "balance_of", (who,)).await;
    result.map_err(|err| format!("Balance query failed: {:?}", err))
}

#[ic_cdk::update]
pub async fn mint_tokens(to: Principal, amount: u64) -> Result<(), String> {
    // ... mint tokens logic ...

    // Call transfer method on icrc1_ledger canister
    transfer(to, amount);

    // Call balance_of method on icrc1_ledger canister
    let balance = balance_of(to);
    println!("Balance of {:#?}: {:#?}", to, balance.await);

    Ok(())
}

#[ic_cdk::update]
pub async fn create_user_record() -> (String, UserRecord) {
    let request_url : String = String::from("https://api-xprnetwork-test.saltant.io/v1/chain/get_table_rows");
    // let request_body : String = String::from(r#"{"json":true,"code":"eosio.token","lower_bound":"XPR","upper_bound":"XPR","table":"accounts","scope":"tommccann","limit":1}"#);
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

