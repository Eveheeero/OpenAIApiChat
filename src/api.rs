use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

// https://platform.openai.com/docs/api-reference/chat/create
pub(super) async fn chat(
    api_key: String,
    machine: Machine,
    messages: &[Message],
    temperature: f64,
) -> Result<Vec<String>, String> {
    let mut body = serde_json::json!({
        "model": machine.to_string(),
        "messages": messages
            .iter()
            .map(|message| {
                serde_json::json!({
                    "role": message.role.to_string(),
                    "content": message.content,
                })
            })
            .collect::<Vec<_>>(),
    });

    // 쿼리 내부 값들을 수정
    {
        let body = body.as_object_mut().unwrap();
        // -2.0부터 2.0까지, 숫자가 높으면 기존 대답을 반복하지 않음
        body.insert(
            "frequency_penalty".into(),
            Value::Number(Number::from_f64(0.0).unwrap()),
        );
        // 특정 토큰이 나타날 가능성 수정, 토큰별로 -1 ~ 1까지, -100 혹은 100은 필수 혹은 밴
        body.insert("logit_bias".into(), Value::Null);
        // 결과 생성 시 사용할 토큰의 최대 수
        // body.insert("max_tokens".into(), Value::Number(Number::from(250)));
        // 결과값 수
        body.insert("n".into(), Value::Number(Number::from(1)));
        // 결과값의 다양성 -2.0부터 2.0까지, 숫자가 높으면 다양성이 높아짐
        body.insert(
            "presence_penalty".into(),
            Value::Number(Number::from_f64(0.0).unwrap()),
        );
        // 샘플링 온도, 0부터 2.0까지, 값이 높아지면 출력값이 비정확해지지만 낮아지면 반복적인 답변, (높아지면 창의성) 기본값 1
        body.insert(
            "temperature".into(),
            Value::Number(Number::from_f64(temperature).unwrap()),
        );
        // temperature 대체, 0.1은 상위 10%정도로 중요한 것만 출력
        body.insert(
            "top_p".into(),
            Value::Number(Number::from_f64(1.0).unwrap()),
        );
    }

    let response = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .expect("openai chat api에 요청을 보내는데 실패하였습니다.");

    let response = response.text().await.unwrap();
    let response = if let Ok(response) = serde_json::from_str::<serde_json::Value>(&response) {
        response
    } else {
        return Err(response);
    };

    if response.get("error").is_some() {
        return Err(response.to_string());
    }

    let mut result = Vec::new();
    for choice in response["choices"]
        .as_array()
        .expect("choices가 배열이 아닙니다.")
    {
        result.push(
            choice["message"]["content"]
                .as_str()
                .expect("content가 문자열이 아닙니다.")
                .to_string(),
        );
    }

    Ok(result)
}

pub(super) struct Message {
    role: Role,
    content: String,
}

impl Message {
    pub(super) fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
    pub(super) fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Role {
    System,
    #[default]
    User,
}

impl Role {
    fn to_string(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
        }
    }
}

// https://platform.openai.com/docs/models/model-endpoint-compatibility
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Default)]
pub(super) enum Machine {
    #[default]
    Gpt35Turbo,
    Gpt4,
    Gpt4Turbo,
    Gpt4O,
    Gpt4OMini,
    GptO1,
    GptO1Mini,
}

impl Machine {
    fn to_string(&self) -> &'static str {
        match self {
            Machine::Gpt35Turbo => "gpt-3.5-turbo",
            Machine::Gpt4 => "gpt-4",
            Machine::Gpt4Turbo => "gpt-4-turbo",
            Machine::Gpt4O => "gpt-4o",
            Machine::Gpt4OMini => "gpt-4o-mini",
            Machine::GptO1 => "o1-preview",
            Machine::GptO1Mini => "o1-mini",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat() {
        let db = sled::open(".chat").expect("데이터베이스를 생성하는데 실패하였습니다.");
        let api_key = db
            .get("api_key")
            .unwrap()
            .map(|x| std::str::from_utf8(&x).unwrap().to_string())
            .unwrap();
        let messages = vec![
            Message {
                role: Role::System,
                content: "You are a helpful assistant.".to_string(),
            },
            Message {
                role: Role::User,
                content: "What is the meaning of life?".to_string(),
            },
        ];
        let result = chat(api_key, Machine::Gpt4OMini, &messages, 1.0)
            .await
            .expect("chat을 호출하는데 실패하였습니다.");
        println!("{:?}", result);
    }
}
