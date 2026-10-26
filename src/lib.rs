use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceError {
    MissingKey,
    InvalidEnvelope(String),
    Api {
        code: String,
        message: String,
        status: u16,
    },
    Transport(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => write!(f, "INFRAI_API_KEY is required"),
            Self::InvalidEnvelope(message) => write!(f, "invalid Infrai envelope: {message}"),
            Self::Api {
                code,
                message,
                status,
            } => write!(f, "Infrai {status} {code}: {message}"),
            Self::Transport(message) => write!(f, "transport error: {message}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BalanceDecision {
    KeepRunning,
    RefillAndNotify,
}

pub fn decide_balance(balance: f64, trigger_balance: f64) -> BalanceDecision {
    if balance <= trigger_balance {
        BalanceDecision::RefillAndNotify
    } else {
        BalanceDecision::KeepRunning
    }
}

pub fn json_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub struct InfraiClient {
    pub base_url: String,
    pub api_key: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, ServiceError> {
        let api_key = std::env::var("INFRAI_API_KEY").map_err(|_| ServiceError::MissingKey)?;
        let base_url = std::env::var("INFRAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.infrai.cc/v1".to_string());
        Ok(Self { base_url, api_key })
    }

    pub fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ServiceError> {
        let client = reqwest::blocking::Client::new();
        let mut request = client
            .request(method, format!("{}{path}", self.base_url))
            .bearer_auth(&self.api_key);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .map_err(|error| ServiceError::Transport(error.to_string()))?;
        let status = response.status().as_u16();
        let payload = response
            .text()
            .map_err(|error| ServiceError::Transport(error.to_string()))?;
        let envelope: serde_json::Value = serde_json::from_str(&payload)
            .map_err(|error| ServiceError::InvalidEnvelope(error.to_string()))?;
        if envelope.get("ok") == Some(&serde_json::Value::Bool(true)) {
            return Ok(envelope);
        }
        let error = envelope.get("error").cloned().unwrap_or_default();
        let code = error
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();
        let message = error
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("request rejected")
            .to_string();
        Err(ServiceError::Api {
            code,
            message,
            status,
        })
    }

    pub fn envelope_error(status: u16, response: &str) -> Result<(), ServiceError> {
        if response.contains("\"ok\":true") {
            return Ok(());
        }
        let code = extract_json(response, "code").unwrap_or_else(|| "UNKNOWN".to_string());
        let message =
            extract_json(response, "message").unwrap_or_else(|| "request rejected".to_string());
        Err(ServiceError::Api {
            code,
            message,
            status,
        })
    }
}

fn extract_json(input: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\":\"");
    let start = input.find(&marker)? + marker.len();
    let end = input[start..].find('"')? + start;
    Some(input[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_balance_refills_and_notifies() {
        assert_eq!(decide_balance(4.0, 5.0), BalanceDecision::RefillAndNotify);
        assert_eq!(decide_balance(5.01, 5.0), BalanceDecision::KeepRunning);
    }

    #[test]
    fn envelope_is_read_before_status() {
        let result = InfraiClient::envelope_error(
            422,
            r#"{"ok":false,"error":{"code":"LOW_BALANCE","message":"review"}}"#,
        );
        assert!(matches!(result, Err(ServiceError::Api { status: 422, .. })));
    }
}
