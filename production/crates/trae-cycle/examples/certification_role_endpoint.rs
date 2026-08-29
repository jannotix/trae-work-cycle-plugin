use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

use serde_json::{Value, json};

fn main() {
    let port = std::env::args()
        .skip(1)
        .find_map(|argument| argument.strip_prefix("--port=").map(str::to_owned))
        .unwrap_or_else(|| "18765".to_owned());
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .expect("certification role endpoint must bind to localhost");
    println!("certification role endpoint: http://127.0.0.1:{port}/v1");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || serve(stream));
            }
            Err(error) => eprintln!("certification endpoint accept failed: {error}"),
        }
    }
}

fn serve(stream: TcpStream) {
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let Some(body) = capture_request(&mut BufReader::new(stream)) else {
        return;
    };
    let content = response_content(&body).to_string();
    let payload = json!({
        "choices": [{
            "finish_reason": "stop",
            "index": 0,
            "message": {"content": content, "role": "assistant"},
        }],
        "id": "chatcmpl-cycle-certification",
        "usage": {"completion_tokens": 32, "prompt_tokens": 64, "total_tokens": 96},
    })
    .to_string();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    let _ = writer.write_all(response.as_bytes());
    let _ = writer.flush();
}

fn capture_request(reader: &mut BufReader<TcpStream>) -> Option<Value> {
    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    if !request_line.starts_with("POST ") {
        return None;
    }
    let mut content_length = 0_usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some(rest) = header.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = rest.trim().parse().ok()?;
        }
    }
    if content_length == 0 || content_length > 4 * 1024 * 1024 {
        return None;
    }
    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn response_content(body: &Value) -> Value {
    let system = body["messages"][0]["content"].as_str().unwrap_or_default();
    let request = body["messages"][1]["content"]
        .as_str()
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .unwrap_or_else(|| json!({}));
    if system.contains("functional_reviewer") {
        return review(&request, "functional_reviewer");
    }
    if system.contains("security_architecture_reviewer") {
        return review(&request, "security_architecture_reviewer");
    }
    if system.contains("You are the arbiter") && system.contains("decide approval") {
        return json!({
            "candidate_digest": request["candidateDigest"],
            "decision": "approved",
            "findings": [],
            "repair_target": null,
            "requirements": requirement_decisions(&request),
        });
    }
    if system.contains("advisory readiness") {
        return json!({
            "blocking": [],
            "next_steps": [],
            "ready": true,
            "summary": "Deterministic certification endpoint reports ready.",
        });
    }
    json!({
        "open_questions": [],
        "points": ["Use one bounded task with the fixture verification command."],
        "risks": [],
        "summary": "Deterministic certification architecture advisory.",
    })
}

fn review(request: &Value, role: &str) -> Value {
    json!({
        "candidate_digest": request["candidateDigest"],
        "decision": "approved",
        "findings": [],
        "repair_target": null,
        "requirements": requirement_decisions(request),
        "role": role,
    })
}

fn requirement_decisions(request: &Value) -> Vec<Value> {
    request["requirementIds"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|id| {
            json!({
                "evidence_ids": request["evidenceIds"].as_array().cloned().unwrap_or_default(),
                "requirement_id": id,
                "status": "satisfied",
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_shapes_bind_candidate_requirements_and_evidence() {
        let request = json!({
            "candidateDigest": "a".repeat(64),
            "evidenceIds": ["e-1"],
            "requirementIds": ["REQ-1"],
        });
        let review = review(&request, "functional_reviewer");
        assert_eq!(review["candidate_digest"], request["candidateDigest"]);
        assert_eq!(review["requirements"][0]["requirement_id"], "REQ-1");
        assert_eq!(review["requirements"][0]["evidence_ids"][0], "e-1");
    }

    #[test]
    fn architect_shape_is_advisory_only() {
        let body = json!({"messages": [
            {"content": "You are the architect", "role": "system"},
            {"content": "{}", "role": "user"},
        ]});
        let response = response_content(&body);
        assert!(response.get("summary").is_some());
        assert!(response.get("decision").is_none());
    }
}
