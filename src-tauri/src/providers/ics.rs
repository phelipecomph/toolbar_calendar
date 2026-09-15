use super::CalendarSource;
use crate::model::{Account, NormalizedEvent};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use icalendar::{Calendar, CalendarComponent, CalendarDateTime, Component, DatePerhapsTime};
use rrule::{RRuleSet, Tz as RTz};

/// Provider de arquivo/URL iCal (ICS). Sem OAuth: só GET + parse.
pub struct IcsSource {
    url: String,
    account_id: String,
    color: String,
}

impl IcsSource {
    pub fn from_account(acc: &Account) -> Result<Self, String> {
        let url = acc
            .config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("conta ICS sem `url` no config")?
            .to_string();
        Ok(Self {
            url,
            account_id: acc.id.clone(),
            color: acc.color.clone(),
        })
    }
}

#[async_trait::async_trait]
impl CalendarSource for IcsSource {
    async fn list_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<NormalizedEvent>, String> {
        // http(s) → GET; qualquer outra coisa → caminho de arquivo local (útil p/ teste).
        let text = if self.url.starts_with("http") {
            reqwest::get(self.url.as_str())
                .await
                .map_err(|e| format!("GET ICS falhou: {e}"))?
                .text()
                .await
                .map_err(|e| format!("corpo ICS inválido: {e}"))?
        } else {
            std::fs::read_to_string(&self.url).map_err(|e| format!("ler ICS local: {e}"))?
        };

        let calendar: Calendar = text.parse().map_err(|e| format!("parse ICS: {e}"))?;

        let mut out = Vec::new();

        for comp in calendar.components.iter() {
            let CalendarComponent::Event(ev) = comp else {
                continue;
            };

            let (Some(start_dt), all_day) = to_utc(ev.get_start()) else {
                continue;
            };
            let (end_opt, _) = to_utc(ev.get_end());
            let end_dt = end_opt.unwrap_or(start_dt);
            let duration = end_dt - start_dt;

            let uid = ev
                .get_uid()
                .map(|s| s.to_string())
                .unwrap_or_else(|| start_dt.timestamp().to_string());
            let title = ev.get_summary().unwrap_or("(sem título)").to_string();
            let description = ev.get_description().map(|s| s.to_string());
            let location = ev.property_value("LOCATION").map(|s| s.to_string());
            let link = ev.property_value("URL").map(|s| s.to_string());

            // Recorrente → expande dentro da janela; senão → instância única.
            let recurring = ev.property_value("RRULE").is_some();
            let starts: Vec<DateTime<Utc>> = if recurring {
                match expand(ev.property_value("RRULE").unwrap(), start_dt, duration, from, to) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[ics] RRULE {uid}: {e} (usando só 1ª instância)");
                        vec![start_dt]
                    }
                }
            } else {
                vec![start_dt]
            };

            for occ_start in starts {
                let occ_end = occ_start + duration;
                if occ_end <= from || occ_start >= to {
                    continue;
                }
                let id = if recurring {
                    format!("{}:{}:{}", self.account_id, uid, occ_start.timestamp())
                } else {
                    format!("{}:{}", self.account_id, uid)
                };
                out.push(NormalizedEvent {
                    id,
                    account_id: self.account_id.clone(),
                    source_id: self.account_id.clone(),
                    title: title.clone(),
                    start: occ_start.to_rfc3339(),
                    end: occ_end.to_rfc3339(),
                    all_day,
                    description: description.clone(),
                    location: location.clone(),
                    meeting_link: link.clone(),
                    html_link: link.clone(),
                    attendees: vec![],
                    color: self.color.clone(),
                });
            }
        }

        Ok(out)
    }
}

/// Expande uma RRULE em ocorrências (UTC) que intersectam [from, to].
/// Usa o DTSTART já convertido p/ UTC (recorrência computada em UTC).
fn expand(
    rrule_val: &str,
    start_dt: DateTime<Utc>,
    duration: Duration,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>, String> {
    let spec = format!(
        "DTSTART:{}\nRRULE:{}",
        start_dt.format("%Y%m%dT%H%M%SZ"),
        rrule_val
    );
    let set: RRuleSet = spec.parse().map_err(|e| format!("{e}"))?;

    // Começa antes da janela por 1 duração p/ pegar ocorrências ainda em andamento.
    let after = (from - duration).with_timezone(&RTz::UTC);
    let before = to.with_timezone(&RTz::UTC);

    let result = set.after(after).before(before).all(500);
    Ok(result
        .dates
        .into_iter()
        .map(|d| d.with_timezone(&Utc))
        .collect())
}

/// Converte DatePerhapsTime -> (UTC, all_day). Retorna (None, false) se ausente/inválido.
fn to_utc(d: Option<DatePerhapsTime>) -> (Option<DateTime<Utc>>, bool) {
    let Some(d) = d else {
        return (None, false);
    };
    match d {
        DatePerhapsTime::Date(date) => (naive_date_to_utc(date), true),
        DatePerhapsTime::DateTime(cdt) => match cdt {
            CalendarDateTime::Utc(dt) => (Some(dt), false),
            CalendarDateTime::Floating(naive) => (Some(naive.and_utc()), false),
            CalendarDateTime::WithTimezone { date_time, tzid } => {
                let dt = tzid
                    .parse::<chrono_tz::Tz>()
                    .ok()
                    .and_then(|tz| tz.from_local_datetime(&date_time).single())
                    .map(|local| local.with_timezone(&Utc));
                (dt, false)
            }
        },
    }
}

fn naive_date_to_utc(date: NaiveDate) -> Option<DateTime<Utc>> {
    Some(date.and_hms_opt(0, 0, 0)?.and_utc())
}
