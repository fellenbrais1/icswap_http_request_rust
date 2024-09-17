// use ic_cdk::api::call::CallResult;
// use candid::{CandidType, Deserialize};
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,
    TransformContext, TransformFunc,
};
use serde::{Serialize, Deserialize};
use serde_json::{self, Value};


// This struct is legacy code and is not really used in the code.
#[derive(Serialize, Deserialize)]
struct Context {
    bucket_start_time_index: usize,
    closing_price_index: usize,
}

// #[ic_cdk::query]
// fn greet(name: String) -> String {
//     format!("Hello, {}!", name)
// }

#[ic_cdk::update]
async fn foo() -> String {
    let request_url : String = String::from("https://api-xprnetwork-main.saltant.io/v1/chain/get_table_rows");
    let request_body : String = String::from(r#"{"json":true,"code":"eosio.token","lower_bound":"XPR","upper_bound":"XPR","table":"accounts","scope":"tommccann","limit":1}"#);

    let host = request_url.split('/').nth(2).unwrap_or_default().to_string();
    let idempotency_key = "d83jf920djc8shf92j8fhs93d82fhs93";
    
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

    // prepare the request_body
    let json_utf8: Vec<u8> = request_body.into_bytes();
    let request_body_vec: Option<Vec<u8>> = Some(json_utf8);

    let request = CanisterHttpRequestArgument {
        url: request_url.clone(),
        method: HttpMethod::POST,
        body: request_body_vec,               //optional for request
        max_response_bytes: None, //optional for request
        // transform: Some(TransformContext {
        //     // The "method" parameter needs to have the same name as the function name of your transform function
        //     function: TransformFunc(candid::Func {
        //         principal: ic_cdk::api::id(),
        //         method: "transform".to_string(),
        //     }),
        //     // The "TransformContext" function does need a context parameter, it can be empty
        //     context: vec![],
        // }),
        transform: None,
        headers: request_headers,
    };

    //Note: in Rust, `http_request()` already sends the cycles needed
    //so no need for explicit Cycles.add() as in Motoko
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
            let str_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            ic_cdk::api::print(format!("{:?}", str_body));

            //The API response will looks like this:
            // { successful: true }

            //Return the body as a string and end the method
            let result: String = format!(
                "{}", str_body
            );
            result
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            //Return the error as a string and end the method
            message
        }
    }

}

