use creator_balance_guard::{decide_balance, BalanceDecision, InfraiClient};

const TRIGGER_BALANCE: f64 = 5.0;
const RECHARGE_AMOUNT: f64 = 25.0;

fn main() {
    let client = match InfraiClient::from_env() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let balance = std::env::var("CREATOR_BALANCE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3.0);
    let balance_response = match client.request(reqwest::Method::GET, "/account/balance", None) {
        Ok(response) => response,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    let live_balance = balance_response
        .pointer("/data/balance")
        .and_then(|value| value.as_f64())
        .unwrap_or(balance);

    match decide_balance(live_balance, TRIGGER_BALANCE) {
        BalanceDecision::KeepRunning => println!("balance={live_balance:.2}; delivery continues"),
        BalanceDecision::RefillAndNotify => {
            let recharge = serde_json::json!({"trigger_balance": TRIGGER_BALANCE, "recharge_amount": RECHARGE_AMOUNT});
            client
                .request(
                    reqwest::Method::PUT,
                    "/account/autorecharge/configure",
                    Some(recharge),
                )
                .unwrap_or_else(|error| {
                    eprintln!("{error}");
                    std::process::exit(1);
                });
            let recipient = std::env::var("INFRAI_EMAIL_TO").unwrap_or_else(|_| {
                eprintln!("INFRAI_EMAIL_TO is required when sending email");
                std::process::exit(2);
            });
            let email = serde_json::json!({"to": recipient, "subject": "Creator balance refilled", "body": "Your delivery queue can continue after the balance refill for creator content."});
            client
                .request(reqwest::Method::POST, "/email/send", Some(email))
                .unwrap_or_else(|error| {
                    eprintln!("{error}");
                    std::process::exit(1);
                });
            println!("refill scheduled; subscriber update queued");
        }
    }
}
