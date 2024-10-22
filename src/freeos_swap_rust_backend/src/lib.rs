use candid::CandidType;
use ic_cdk::api::management_canister::http_request::{http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,TransformContext, TransformFunc};
use serde::{Serialize, Deserialize};
use serde_json::{self, Value};
use std::vec;

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct UserRecord {
    proton_account: String,
    ic_principal: String,
    amount: f32,
    utc_time: u64,
}

#[ic_cdk::update]
pub async fn create_user_record() -> String {
    let request_url : String = String::from("https://api-xprnetwork-test.saltant.io/v1/chain/get_table_rows");
    let request_body : String = String::from(r#"{"json":true,"code":"freeosgov2","lower_bound":1726732990,"upper_bound":1726735767,"table":"swaps","scope":"freeosgov2","limit":100}"#);

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
            match serde_json::from_str::<Value>(str_body) {
                Ok(parsed) => {
                    if let Some(rows) = parsed["rows"].as_array() {
                        let mut counter = 0;
                        for row in rows {
                    // Access the second record in the array and retrieve the Amount
                            if let Some(proton_account) = row["proton_account"].as_str() {
                                ic_cdk::api::print(format!("Proton Account from record {}: {}", counter, proton_account));
                            } else {
                                ic_cdk::api::print(format!("Failed to retrieve 'proton_account' from the row"));
                            }
                            if let Some(ic_principal) = row["ic_principal"].as_str() {
                                ic_cdk::api::print(format!("IC Principal from record {}: {}", counter, ic_principal));
                            } else {
                                ic_cdk::api::print(format!("Failed to retrieve 'ic_principal' from the row"));
                            }
                            if let Some(amount) = row["amount"].as_str() {
                                let index = amount.find(" ");

                                if let Some(index) = index {
                                    let number_str = &amount[..index];
                                    let number: f32 = number_str.parse().unwrap();
                                    ic_cdk::api::print(format!("Amount from record {}: {}", counter, number));
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
        
            //The API response will looks like this:
            // { successful: true }

            //Return the body as a string and end the method
            let result: String = format!(
                "{}", str_body
            );

            return result
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            //Return the error as a string and end the method
            return message
        }
    }
}

#[ic_cdk::query]
fn clean_dynamic_content(args: TransformArgs) -> HttpResponse {
    let mut response = args.response;

    // Filter out the 'Date' header from the headers
    response.headers.clear();
    
    // Return the cleaned response
    response
}