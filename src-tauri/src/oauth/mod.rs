pub mod tokens;

use base64::Engine;
use sha2::{Digest, Sha256};

const AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";

pub struct Tokens {
    #[allow(dead_code)]
    pub access_token: String,
    pub refresh_token: Option<String>,
    #[allow(dead_code)]
    pub expires_in: i64,
}

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_string(n: usize) -> String {
    let mut buf = vec![0u8; n];
    getrandom::getrandom(&mut buf).expect("getrandom falhou");
    b64url(&buf)
}

fn pkce() -> (String, String) {
    let verifier = random_string(48); // ~64 chars base64url
    let digest = Sha256::digest(verifier.as_bytes());
    (verifier, b64url(digest.as_ref()))
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// Fluxo interativo PKCE + loopback (BLOQUEANTE — rodar numa thread própria,
/// nunca dentro do runtime tokio: usa reqwest::blocking).
pub fn authorize(client_id: &str, client_secret: &str) -> Result<Tokens, String> {
    let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = server
        .server_addr()
        .to_ip()
        .ok_or("sem porta loopback")?
        .port();
    let redirect = format!("http://127.0.0.1:{port}");
    let (verifier, challenge) = pkce();
    let state = random_string(16);

    let url = format!(
        "{AUTH}?response_type=code&client_id={cid}&redirect_uri={ru}&scope={scope}\
         &code_challenge={ch}&code_challenge_method=S256&state={st}&access_type=offline&prompt=consent",
        cid = urlenc(client_id),
        ru = urlenc(&redirect),
        scope = urlenc(SCOPE),
        ch = challenge,
        st = state,
    );

    webbrowser::open(&url).map_err(|e| format!("abrir navegador: {e}"))?;

    // Espera o redirect do Google.
    let request = server.recv().map_err(|e| e.to_string())?;
    let raw = request.url().to_string(); // "/?code=...&state=..."
    let (code, got_state) = parse_callback(&raw);
    if got_state.as_deref() != Some(state.as_str()) {
        let _ = request.respond(tiny_http::Response::from_string("state OAuth inválido."));
        return Err("state OAuth não confere".into());
    }
    let code = code.ok_or("callback sem `code`")?;
    let _ = request.respond(tiny_http::Response::from_string(
        "Autenticado no Agenda Strip. Pode fechar esta aba.",
    ));

    // Troca code -> tokens (blocking).
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code.as_str()),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect.as_str()),
        ("code_verifier", verifier.as_str()),
    ];
    let json: serde_json::Value = reqwest::blocking::Client::new()
        .post(TOKEN)
        .form(&params)
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("resposta de token sem access_token: {json}"))?
        .to_string();
    let refresh_token = json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let expires_in = json.get("expires_in").and_then(|v| v.as_i64()).unwrap_or(3600);

    Ok(Tokens {
        access_token,
        refresh_token,
        expires_in,
    })
}

/// Refresh assíncrono (usado pelo provider a cada sync).
pub async fn refresh(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<String, String> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];
    let json: serde_json::Value = reqwest::Client::new()
        .post(TOKEN)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    json.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("refresh sem access_token: {json}"))
}

fn parse_callback(raw: &str) -> (Option<String>, Option<String>) {
    let q = raw.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut code = None;
    let mut state = None;
    for (k, v) in url::form_urlencoded::parse(q.as_bytes()) {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state = Some(v.into_owned()),
            _ => {}
        }
    }
    (code, state)
}
