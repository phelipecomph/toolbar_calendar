use super::CalendarSource;
use crate::model::{Account, Attendee, NormalizedEvent};
use crate::oauth;
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;

pub struct GoogleSource {
    account_id: String,
    fallback_color: String,
    client_id: String,
    client_secret: String,
    refresh_token: String,
}

impl GoogleSource {
    pub fn from_account(acc: &Account) -> Result<Self, String> {
        let cfg = &acc.config;
        let client_id = cfg
            .get("client_id")
            .and_then(|v| v.as_str())
            .ok_or("conta Google sem client_id")?
            .to_string();
        let client_secret = cfg
            .get("client_secret")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let refresh_token =
            oauth::tokens::load_refresh(&acc.id).map_err(|e| format!("keyring: {e}"))?;
        Ok(Self {
            account_id: acc.id.clone(),
            fallback_color: acc.color.clone(),
            client_id,
            client_secret,
            refresh_token,
        })
    }
}

#[async_trait::async_trait]
impl CalendarSource for GoogleSource {
    async fn list_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<NormalizedEvent>, String> {
        let access =
            oauth::refresh(&self.client_id, &self.client_secret, &self.refresh_token).await?;
        let client = reqwest::Client::new();

        // 1. Todas as agendas visíveis na conta (próprias + compartilhadas).
        let cal_list: Value = client
            .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
            .bearer_auth(&access)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        if let Some(err) = cal_list.get("error") {
            return Err(format!("google calendarList: {err}"));
        }

        let calendars = cal_list
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        eprintln!("[google] {} agenda(s) na conta", calendars.len());

        // 2. Eventos de cada agenda; cor = backgroundColor da agenda (ou cor da conta).
        let mut out = Vec::new();
        for cal in &calendars {
            let Some(cal_id) = cal.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let name = cal
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or(cal_id);
            let color = cal
                .get("backgroundColor")
                .and_then(|v| v.as_str())
                .unwrap_or(&self.fallback_color)
                .to_string();

            match self
                .fetch_events(&client, &access, cal_id, name, &color, from, to)
                .await
            {
                Ok(evs) => {
                    eprintln!("[google]   \"{name}\": {} evento(s) na janela", evs.len());
                    out.extend(evs);
                }
                Err(e) => eprintln!("[google]   \"{name}\": ERRO {e}"),
            }
        }

        Ok(out)
    }
}

impl GoogleSource {
    async fn fetch_events(
        &self,
        client: &reqwest::Client,
        access: &str,
        cal_id: &str,
        cal_name: &str,
        color: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<NormalizedEvent>, String> {
        let endpoint = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            urlenc(cal_id)
        );
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = client.get(&endpoint).bearer_auth(access).query(&[
                ("timeMin", from.to_rfc3339()),
                ("timeMax", to.to_rfc3339()),
                ("singleEvents", "true".to_string()),
                ("orderBy", "startTime".to_string()),
                ("maxResults", "2500".to_string()),
            ]);
            if let Some(t) = &page_token {
                req = req.query(&[("pageToken", t.as_str())]);
            }

            let json: Value = req
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;

            if let Some(err) = json.get("error") {
                return Err(format!("{err}"));
            }
            if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
                for it in items {
                    if let Some(ev) = map_event(it, &self.account_id, cal_name, color) {
                        out.push(ev);
                    }
                }
            }
            match json.get("nextPageToken").and_then(|v| v.as_str()) {
                Some(t) => page_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(out)
    }
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn map_event(it: &Value, account_id: &str, cal_name: &str, color: &str) -> Option<NormalizedEvent> {
    if it.get("status").and_then(|v| v.as_str()) == Some("cancelled") {
        return None;
    }
    let id_g = it.get("id")?.as_str()?;
    let (start, all_day) = parse_gdt(it.get("start")?)?;
    let (end, _) = parse_gdt(it.get("end")?)?;

    // Título: usa o do evento; se vazio, cai pro nome da agenda.
    let title = it
        .get("summary")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(cal_name)
        .to_string();

    // Descrição: prefixa o nome da agenda, mantendo a descrição original abaixo.
    let description = match it.get("description").and_then(|v| v.as_str()) {
        Some(d) if !d.trim().is_empty() => Some(format!("Agenda: {cal_name}\n\n{d}")),
        _ => Some(format!("Agenda: {cal_name}")),
    };

    let meeting_link = it
        .get("hangoutLink")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            it.get("conferenceData")
                .and_then(|c| c.get("entryPoints"))
                .and_then(|e| e.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find_map(|ep| ep.get("uri").and_then(|u| u.as_str()))
                        .map(|s| s.to_string())
                })
        });

    let attendees = it
        .get("attendees")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some(Attendee {
                        email: a.get("email").and_then(|v| v.as_str())?.to_string(),
                        name: a
                            .get("displayName")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        status: a
                            .get("responseStatus")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(NormalizedEvent {
        id: format!("{account_id}:{id_g}"),
        account_id: account_id.to_string(),
        source_id: account_id.to_string(),
        title,
        start,
        end,
        all_day,
        description,
        location: it
            .get("location")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        meeting_link,
        html_link: it
            .get("htmlLink")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        attendees,
        color: color.to_string(),
    })
}

/// Google start/end: `dateTime` (RFC3339 c/ offset) ou `date` (all-day). Normaliza p/ UTC.
fn parse_gdt(v: &Value) -> Option<(String, bool)> {
    if let Some(dt) = v.get("dateTime").and_then(|x| x.as_str()) {
        let parsed = DateTime::parse_from_rfc3339(dt).ok()?;
        Some((parsed.with_timezone(&Utc).to_rfc3339(), false))
    } else if let Some(d) = v.get("date").and_then(|x| x.as_str()) {
        let date = NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()?;
        Some((date.and_hms_opt(0, 0, 0)?.and_utc().to_rfc3339(), true))
    } else {
        None
    }
}
